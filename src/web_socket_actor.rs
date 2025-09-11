use crate::{
    error::Result,
    file_change_data::{FileAddData, FileChangeData, FileRemoveData},
};
use axum::extract::ws::{CloseFrame, Message, WebSocket, close_code};
use itertools::Itertools;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::instrument;

#[derive(Debug)]
pub enum WebSocketActorEvent {
    Change(FileChangeData),
}

#[derive(Debug)]
pub struct WebSocketActor {
    ws: WebSocket,
    file_add_chunk_size: usize,
    file_add_chunk_delay: Duration,
}

impl WebSocketActor {
    pub fn new(ws: WebSocket, file_add_chunk_size: usize, file_add_chunk_delay: Duration) -> Self {
        Self {
            ws,
            file_add_chunk_size,
            file_add_chunk_delay,
        }
    }

    async fn ws_send_change(&mut self, change: FileChangeData) -> Result<()> {
        Ok(self
            .ws
            .send(Message::Text(serde_json::to_string(&change)?.into()))
            .await?)
    }

    async fn ws_send_change_chunked(&mut self, change: FileChangeData) -> Result<()> {
        let adds = change.added.0;
        let removes = change.removed.0;

        let mut multi_adds: Vec<Vec<_>> = adds
            .into_iter()
            .chunks(self.file_add_chunk_size)
            .into_iter()
            .map(|chunk| chunk.collect())
            .collect();

        let change = FileChangeData {
            removed: FileRemoveData(removes),
            added: FileAddData(if multi_adds.is_empty() {
                Vec::new()
            } else {
                multi_adds.remove(0)
            }),
        };
        self.ws_send_change(change).await?;

        for chunk in multi_adds.drain(..) {
            tokio::time::sleep(self.file_add_chunk_delay).await;
            let change = FileChangeData {
                removed: FileRemoveData(Vec::new()),
                added: FileAddData(chunk),
            };
            self.ws_send_change(change).await?;
        }

        Ok(())
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
                            let result = self.ws_send_change_chunked(change).await;
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
