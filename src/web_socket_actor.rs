use crate::{
    authentication::{
        Token,
        authentication_token_store_actor::{
            AuthenticationTokenStoreActor, AuthenticationTokenStoreActorEvent,
        },
    },
    error::Result,
    file_change_data::FileChangeData,
};
use axum::extract::ws::{CloseFrame, Message, WebSocket, close_code};
use tokio::sync::mpsc;
use tracing::instrument;

#[derive(Debug)]
pub enum WebSocketActorEvent {
    Change(FileChangeData),
}

#[derive(Debug)]
pub struct WebSocketActor {
    ws: WebSocket,
    authentication_token_store_actor_sender: mpsc::Sender<AuthenticationTokenStoreActorEvent>,
    token_refresh_timer: tokio::time::Interval,
    token: Token,
}

impl WebSocketActor {
    pub fn new(
        ws: WebSocket,
        authentication_token_store_actor_sender: mpsc::Sender<AuthenticationTokenStoreActorEvent>,
        token_refresh_interval: std::time::Duration,
        token: Token,
    ) -> Self {
        let mut token_refresh_timer = tokio::time::interval(token_refresh_interval);
        token_refresh_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        Self {
            ws,
            authentication_token_store_actor_sender,
            token_refresh_timer,
            token,
        }
    }

    async fn ws_send_change(&mut self, change: FileChangeData) -> Result<()> {
        Ok(self
            .ws
            .send(Message::Text(serde_json::to_string(&change)?.into()))
            .await?)
    }

    fn ws_send_close_frame(
        &mut self,
    ) -> impl Future<Output = std::result::Result<(), axum::Error>> {
        self.ws.send(Message::Close(Some(CloseFrame {
            code: close_code::AWAY,
            reason: "".into(),
        })))
    }

    #[instrument]
    pub async fn run(mut self, mut receiver: mpsc::Receiver<WebSocketActorEvent>) {
        tracing::debug!("actor started");
        loop {
            tokio::select! {
                msg = receiver.recv() => {
                    match msg {
                        Some(WebSocketActorEvent::Change(change)) => {
                            let result = self.ws_send_change(change).await;
                            if let Err(err) = result {
                                tracing::error!("failed to send change: {}", err);
                                break;
                            }
                        },
                        None => {
                            let _ = self.ws_send_close_frame().await.inspect_err(|e| tracing::warn!("failed to send close frame: {}", e));
                            break;
                        },
                    }
                },
                msg = self.ws.recv() => {
                    if msg.is_none() {
                        tracing::info!("websocket closed");
                        break;
                    }
                },
                _ = self.token_refresh_timer.tick() => {
                    let result = AuthenticationTokenStoreActor::check_and_refresh_token(&mut self.authentication_token_store_actor_sender, self.token.clone()).await;
                    if !result.inspect_err(|e| tracing::error!("failed to refresh token: {}", e)).unwrap_or(false) {
                        break;
                    }
                }
            }
        }
        tracing::debug!("actor stopped");
    }

    pub async fn send_change(
        sender: &mpsc::Sender<WebSocketActorEvent>,
        change: FileChangeData,
    ) -> Result<()> {
        sender.send(WebSocketActorEvent::Change(change)).await?;
        Ok(())
    }
}
