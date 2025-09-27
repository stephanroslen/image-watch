use crate::authentication::{Deadline, Token, Username};
use tokio::{
    sync::{mpsc, oneshot},
    time::{Interval, MissedTickBehavior},
};

pub enum AuthenticationTokenStoreActorEvent {
    CheckAndRefreshToken {
        token: Token,
        response_sender: oneshot::Sender<bool>,
    },
    GetToken {
        username: Username,
        response_sender: oneshot::Sender<Token>,
    },
    RevokeToken {
        token: Token,
    },
}

pub struct AuthenticationTokenStoreActor {
    tokens: std::collections::HashMap<Token, Username>,
    token_deadlines:
        std::collections::HashMap<Username, std::collections::HashMap<Token, Deadline>>,
    cleanup_timer: Interval,
    auth_token_ttl: std::time::Duration,
    auth_token_max_per_user: usize,
}

impl AuthenticationTokenStoreActor {
    fn do_check_and_refresh_token(&mut self, token: Token) -> bool {
        if let Some(username) = self.tokens.get(&token) {
            self.token_deadlines
                .entry(username.clone())
                .or_default()
                .insert(token, Self::make_deadline(self.auth_token_ttl));
            return true;
        }
        false
    }

    fn make_deadline(auth_token_ttl: std::time::Duration) -> Deadline {
        Deadline(std::time::Instant::now() + auth_token_ttl)
    }

    async fn remove_token(&mut self, token: Token) {
        self.tokens.remove(&token);
    }

    async fn cleanup(&mut self) {
        let now = std::time::Instant::now();

        for (_, tokens) in self.token_deadlines.iter_mut() {
            let mut survivors = Vec::new();
            for (token, deadline) in tokens.drain() {
                if deadline.0 < now {
                    self.tokens.remove(&token);
                } else {
                    survivors.push((token, deadline));
                }
            }
            if survivors.len() >= self.auth_token_max_per_user {
                survivors.sort_by_key(|(_, deadline)| deadline.0);
                for (token, _) in survivors.drain(self.auth_token_max_per_user..) {
                    self.tokens.remove(&token);
                }
            }
            *tokens = survivors.drain(..).collect();
        }

        self.token_deadlines.retain(|_, tokens| !tokens.is_empty());
    }

    async fn do_get_token(&mut self, username: Username) -> Token {
        let token = Token::generate();
        self.tokens.insert(token.clone(), username.clone());
        self.token_deadlines
            .entry(username)
            .or_default()
            .insert(token.clone(), Self::make_deadline(self.auth_token_ttl));
        token
    }

    pub async fn run(mut self, mut receiver: mpsc::Receiver<AuthenticationTokenStoreActorEvent>) {
        tracing::debug!("actor started");
        loop {
            tokio::select! {
                msg = receiver.recv() => match msg {
                    Some(msg) => {
                        match msg {
                            AuthenticationTokenStoreActorEvent::CheckAndRefreshToken { token, response_sender} => {
                              let _ = response_sender
                                    .send(self.do_check_and_refresh_token(token.clone()))
                                    .inspect_err(|e| {tracing::error!("Error responding to AuthenticatorEvent::RefreshToken: {:?}", e)});

                            },
                            AuthenticationTokenStoreActorEvent::GetToken{username, response_sender} => {
                                let _ = response_sender.send(self.do_get_token(username).await).inspect_err(|e| {tracing::error!("Error responding to AuthenticatorEvent::GetToken: {:?}", e)});
                            },
                            AuthenticationTokenStoreActorEvent::RevokeToken { token } => {
                                self.remove_token(token).await;
                            }
                        }
                    },
                    None => break,
                },
                _ = self.cleanup_timer.tick() => {
                    self.cleanup().await;
                }
            }
        }
        tracing::debug!("stopped");
    }

    pub async fn check_and_refresh_token(
        sender: &mut mpsc::Sender<AuthenticationTokenStoreActorEvent>,
        token: Token,
    ) -> crate::error::Result<bool> {
        let (response_sender, response_receiver) = oneshot::channel();
        let message = AuthenticationTokenStoreActorEvent::CheckAndRefreshToken {
            token,
            response_sender,
        };
        sender.send(message).await?;
        Ok(response_receiver.await?)
    }

    pub async fn get_token(
        sender: &mut mpsc::Sender<AuthenticationTokenStoreActorEvent>,
        username: Username,
    ) -> crate::error::Result<Token> {
        let (response_sender, response_receiver) = oneshot::channel();
        let message = AuthenticationTokenStoreActorEvent::GetToken {
            username,
            response_sender,
        };
        sender.send(message).await?;
        Ok(response_receiver.await?)
    }

    pub async fn revoke_token(
        sender: mpsc::Sender<AuthenticationTokenStoreActorEvent>,
        token: Token,
    ) -> crate::error::Result<()> {
        let message = AuthenticationTokenStoreActorEvent::RevokeToken { token };
        sender.send(message).await?;
        Ok(())
    }

    pub fn new(
        auth_token_cleanup_interval: std::time::Duration,
        auth_token_ttl: std::time::Duration,
        auth_token_max_per_user: usize,
    ) -> Self {
        let tokens = std::collections::HashMap::new();
        let token_deadlines = std::collections::HashMap::new();
        let mut cleanup_timer = tokio::time::interval(auth_token_cleanup_interval);
        // continue with intended interval even if the timer is missed
        cleanup_timer.set_missed_tick_behavior(MissedTickBehavior::Delay);
        Self {
            tokens,
            token_deadlines,
            cleanup_timer,
            auth_token_ttl,
            auth_token_max_per_user,
        }
    }
}
