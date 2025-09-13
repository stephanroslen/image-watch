use argon2::{Argon2, PasswordHash, PasswordVerifier, password_hash::Error};
use axum::{
    body::Body,
    http::{HeaderMap, Request, StatusCode, Uri, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::instrument;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Token(pub String);

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
    RevokeToken {
        token: Token,
    },
}

#[derive(Debug)]
pub struct AuthenticationActor {
    tokens: std::collections::HashSet<Token>,
    username: String,
    password_argon2: String,
}

impl AuthenticationActor {
    pub fn new(username: String, password_argon2: String) -> Self {
        let tokens = std::collections::HashSet::new();
        Self {
            tokens,
            username,
            password_argon2,
        }
    }

    fn verify_password(hash: &str, password: &str) -> Result<bool, Error> {
        let parsed_hash = PasswordHash::new(hash)?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    async fn authenticate_request(&self, token: Option<Token>, uri: Uri) -> bool {
        let path = uri.path();
        // TODO: more flexible check
        if !path.starts_with("/backend") || path == "/backend/login" {
            return true;
        }
        if let Some(token) = token {
            return self.tokens.contains(&token);
        }
        false
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
            self.tokens.insert(token.clone());
            Some(token)
        } else {
            None
        }
    }

    async fn remove_token(&mut self, token: Token) {
        self.tokens.remove(&token);
    }

    #[instrument]
    pub async fn run(mut self, mut receiver: mpsc::Receiver<AuthenticationActorEvent>) {
        tracing::debug!("actor started");
        while let Some(msg) = receiver.recv().await {
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
                AuthenticationActorEvent::RevokeToken { token } => {
                    self.remove_token(token).await;
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
            let token = Self::extract_token(&req);

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

    pub fn extract_token(req: &Request<Body>) -> Option<Token> {
        let headers: &HeaderMap = req.headers();

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
