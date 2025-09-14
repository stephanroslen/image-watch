use argon2::{Argon2, PasswordHash, PasswordVerifier, password_hash::Error};
use axum::http::HeaderValue;
use axum::{
    body::Body,
    http::{HeaderMap, Request, StatusCode, Uri, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use tokio::{
    sync::mpsc,
    time::{Interval, MissedTickBehavior},
};
use tracing::instrument;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Token(pub String);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Username(String);

#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
struct Deadline(std::time::Instant);

impl Token {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().into())
    }
}

#[derive(Debug, Deserialize)]
pub struct Credentials {
    username: String,
    password: String,
}

#[derive(Debug)]
pub enum AuthenticationActorEvent {
    AuthenticateRequest {
        token: Option<Token>,
        uri: Uri,
        response_sender: tokio::sync::oneshot::Sender<bool>,
    },
    GetToken {
        credentials: Credentials,
        response_sender: tokio::sync::oneshot::Sender<Option<Token>>,
    },
    RefreshToken {
        token: Token,
        response_sender: tokio::sync::oneshot::Sender<bool>,
    },
    RevokeToken {
        token: Token,
    },
}

#[derive(Debug)]
pub struct AuthenticationActor {
    tokens: std::collections::HashMap<Token, Username>,
    token_deadlines:
        std::collections::HashMap<Username, std::collections::HashMap<Token, Deadline>>,
    username: String,
    password_argon2: String,
    cleanup_timer: Interval,
    auth_token_ttl: std::time::Duration,
    auth_token_max_per_user: usize,
}

impl AuthenticationActor {
    pub fn new(
        username: String,
        password_argon2: String,
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
            username,
            password_argon2,
            cleanup_timer,
            auth_token_ttl,
            auth_token_max_per_user,
        }
    }

    fn verify_password(hash: &str, password: &str) -> Result<bool, Error> {
        let parsed_hash = PasswordHash::new(hash)?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    async fn authenticate_request(&mut self, token: Option<Token>, uri: Uri) -> bool {
        let path = uri.path();
        // TODO: more flexible check
        if !path.starts_with("/backend")
            || path == "/backend/login"
            || path == "/backend/frontend_hash"
        {
            return true;
        }
        if let Some(token) = token {
            return self.check_and_refresh_token(token);
        }
        false
    }

    fn check_and_refresh_token(&mut self, token: Token) -> bool {
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

    async fn authenticate(
        &mut self,
        Credentials { username, password }: Credentials,
    ) -> Option<Token> {
        if username == self.username
            && Self::verify_password(&self.password_argon2, &password)
                .inspect_err(|e| tracing::error!("Error verifying password: {:?}", e))
                .unwrap_or(false)
        {
            let token = Token::generate();
            self.tokens
                .insert(token.clone(), Username(username.clone()));
            self.token_deadlines
                .entry(Username(username))
                .or_default()
                .insert(token.clone(), Self::make_deadline(self.auth_token_ttl));
            Some(token)
        } else {
            None
        }
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

    #[instrument]
    pub async fn run(mut self, mut receiver: mpsc::Receiver<AuthenticationActorEvent>) {
        tracing::debug!("actor started");
        loop {
            tokio::select! {
                msg = receiver.recv() => match msg {
                    Some(msg) => {
                        match msg {
                            AuthenticationActorEvent::AuthenticateRequest {
                                token,
                                uri,
                                response_sender: response,
                            } => {
                                let _ = response
                                    .send(self.authenticate_request(token, uri).await)
                                    .inspect_err(|e| {
                                        tracing::error!(
                                            "Error responding to AuthenticatorEvent::VerifyToken: {:?}",
                                            e
                                        )
                                    });
                            }
                            AuthenticationActorEvent::GetToken {
                                credentials,
                                response_sender: response,
                            } => {
                                let _ = response
                                    .send(self.authenticate(credentials).await)
                                    .inspect_err(|e| {
                                        tracing::error!(
                                            "Error responding to AuthenticatorEvent::Authenticate: {:?}",
                                            e
                                        )
                                    });
                            }
                            AuthenticationActorEvent::RefreshToken { token , response_sender: response} => {
                                let _ = response
                                    .send(self.check_and_refresh_token(token.clone()))
                                    .inspect_err(|e| {tracing::error!("Error responding to AuthenticatorEvent::RefreshToken: {:?}", e)});
                            }
                            AuthenticationActorEvent::RevokeToken { token } => {
                                self.remove_token(token).await;
                            }
                        }},
                    None => break,
                },
                _ = self.cleanup_timer.tick() => {
                    self.cleanup().await;
                }
            }
        }
        tracing::debug!("stopped");
    }

    pub async fn auth_request(
        sender: mpsc::WeakSender<AuthenticationActorEvent>,
        req: Request<Body>,
        next: Next,
    ) -> Result<Response, Response> {
        if let Some(sender) = sender.upgrade() {
            let token = Self::extract_token(&req.headers());

            let (response_sender, response_receiver) = tokio::sync::oneshot::channel();

            let uri = req.uri().clone();

            if let Ok(_) = sender
                .send(AuthenticationActorEvent::AuthenticateRequest {
                    token,
                    uri,
                    response_sender,
                })
                .await
                && let Ok(authenticated) = response_receiver.await
                && authenticated
            {
                return Ok(next.run(req).await);
            }
        } else {
            let resp = (StatusCode::SERVICE_UNAVAILABLE, "Service restarting").into_response();
            return Err(resp);
        }
        let resp = (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
        Err(resp)
    }

    pub async fn get_token(
        sender: mpsc::Sender<AuthenticationActorEvent>,
        credentials: Credentials,
    ) -> crate::error::Result<Option<Token>> {
        let (response_sender, response_receiver) = tokio::sync::oneshot::channel();

        sender
            .send(AuthenticationActorEvent::GetToken {
                credentials,
                response_sender,
            })
            .await?;
        Ok(response_receiver.await?)
    }

    pub async fn revoke_token(
        sender: mpsc::Sender<AuthenticationActorEvent>,
        token: Token,
    ) -> crate::error::Result<()> {
        sender
            .send(AuthenticationActorEvent::RevokeToken { token })
            .await?;
        Ok(())
    }

    pub async fn refresh_token(
        sender: mpsc::Sender<AuthenticationActorEvent>,
        token: Token,
    ) -> crate::error::Result<bool> {
        let (response_sender, response_receiver) = tokio::sync::oneshot::channel();
        sender
            .send(AuthenticationActorEvent::RefreshToken {
                token,
                response_sender,
            })
            .await?;
        Ok(response_receiver.await?)
    }

    pub fn extract_token(headers: &HeaderMap<HeaderValue>) -> Option<Token> {
        headers
            .get(header::AUTHORIZATION)
            .and_then(|auth_header| auth_header.to_str().ok())
            .and_then(|auth_str| auth_str.strip_prefix("Bearer "))
            .map(|token| Token(token.to_string()))
            .or_else(|| {
                headers
                    .get(header::SEC_WEBSOCKET_PROTOCOL)
                    .and_then(|auth_header| auth_header.to_str().ok())
                    .and_then(|auth_str| auth_str.strip_prefix("bearer, "))
                    .map(|token| Token(token.to_string()))
            })
    }
}
