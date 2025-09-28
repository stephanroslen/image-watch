use crate::authentication::{
    Token, Username,
    authentication_token_store_actor::{
        AuthenticationTokenStoreActor, AuthenticationTokenStoreActorEvent,
    },
};
use argon2::{Argon2, PasswordHash, PasswordVerifier, password_hash::Error};
use axum::{
    body::Body,
    http::{HeaderMap, HeaderValue, Request, StatusCode, Uri, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::instrument;

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
}

#[derive(Debug)]
pub struct AuthenticationActor {
    username: String,
    password_argon2: String,
    authentication_token_store_actor_sender: mpsc::Sender<AuthenticationTokenStoreActorEvent>,
}

impl AuthenticationActor {
    pub fn new(
        username: String,
        password_argon2: String,
        authentication_token_store_actor_sender: mpsc::Sender<AuthenticationTokenStoreActorEvent>,
    ) -> Self {
        Self {
            username,
            password_argon2,
            authentication_token_store_actor_sender,
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
            return AuthenticationTokenStoreActor::check_and_refresh_token(
                &mut self.authentication_token_store_actor_sender,
                token,
            )
            .await
            .unwrap_or(false);
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
            AuthenticationTokenStoreActor::get_token(
                &mut self.authentication_token_store_actor_sender,
                Username(username),
            )
            .await
            .ok()
        } else {
            None
        }
    }

    #[instrument(level = "trace")]
    pub async fn run(mut self, mut receiver: mpsc::Receiver<AuthenticationActorEvent>) {
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
                        }},
                    None => break,
                },
            }
        }
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
