use crate::file_change_data::{FileAddData, FileChangeData, FileRemoveData};
use axum::extract::ws::{Message, WebSocket};
use itertools::Itertools;
use std::time::Duration;
use tokio::{sync::mpsc, task::JoinHandle};
use tracing::instrument;

#[derive(Debug)]
enum WebSocketActorEvent {
    Change(FileChangeData),
}

#[derive(Debug)]
struct WebSocketActor {
    receiver: mpsc::Receiver<WebSocketActorEvent>,
    ws: WebSocket,
    file_add_chunk_size: usize,
    file_add_chunk_delay: Duration,
}

impl WebSocketActor {
    fn new(
        receiver: mpsc::Receiver<WebSocketActorEvent>,
        ws: WebSocket,
        file_add_chunk_size: usize,
        file_add_chunk_delay: Duration,
    ) -> Self {
        Self {
            receiver,
            ws,
            file_add_chunk_size,
            file_add_chunk_delay,
        }
    }

    async fn send_change(&mut self, change: FileChangeData) -> crate::error::Result<()> {
        Ok(self
            .ws
            .send(Message::Text(serde_json::to_string(&change)?.into()))
            .await?)
    }

    async fn send_change_chunked(&mut self, change: FileChangeData) -> crate::error::Result<()> {
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
        self.send_change(change).await?;

        for chunk in multi_adds.drain(..) {
            tokio::time::sleep(self.file_add_chunk_delay).await;
            let change = FileChangeData {
                removed: FileRemoveData(Vec::new()),
                added: FileAddData(chunk),
            };
            self.send_change(change).await?;
        }

        Ok(())
    }

    #[instrument]
    async fn run(mut self) {
        tracing::debug!("actor started");
        loop {
            tokio::select! {
                msg = self.receiver.recv() => {
                    match msg {
                        Some(WebSocketActorEvent::Change(change)) => {
                            let result = self.send_change_chunked(change).await;
                            if let Err(err) = result {
                                tracing::error!("failed to send change: {}", err);
                                break;
                            }
                        },
                        None => {
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
}

#[derive(Debug)]
pub struct WebSocketActorHandler {
    sender: mpsc::Sender<WebSocketActorEvent>,
    join_handle: JoinHandle<()>,
}

impl WebSocketActorHandler {
    pub fn new(ws: WebSocket, file_add_chunk_size: usize, file_add_chunk_delay: Duration) -> Self {
        let (tx, rx) = mpsc::channel::<WebSocketActorEvent>(8);
        let actor = WebSocketActor::new(rx, ws, file_add_chunk_size, file_add_chunk_delay);
        let join_handle = tokio::spawn(actor.run());

        Self {
            sender: tx,
            join_handle,
        }
    }

    pub async fn send_change(&self, change: FileChangeData) -> crate::error::Result<()> {
        self.sender
            .send(WebSocketActorEvent::Change(change))
            .await?;
        Ok(())
    }

    pub fn join_handle(self) -> JoinHandle<()> {
        self.join_handle
    }
}
