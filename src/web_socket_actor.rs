use crate::{error::Result, file_change_data::FileChangeData};
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
}

impl WebSocketActor {
    pub fn new(ws: WebSocket) -> Self {
        Self { ws }
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
