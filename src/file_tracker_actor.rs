use crate::file_change_data::{FileAddData, FileChangeData, FileRemoveData};
use crate::shutdown_actor::ShutdownActorHandler;
use crate::web_socket_actor::WebSocketActorHandler;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::instrument;

#[derive(Debug)]
enum FileTrackerActorEvent {
    Change(FileChangeData),
    AddWebSocketActorHandler(WebSocketActorHandler),
}

#[derive(Debug)]
struct FileTrackerActor {
    receiver: mpsc::Receiver<FileTrackerActorEvent>,
    baseline: FileAddData,
    web_socket_actor_handlers: Vec<WebSocketActorHandler>,
}

impl FileTrackerActor {
    fn new(receiver: mpsc::Receiver<FileTrackerActorEvent>) -> Self {
        let baseline = FileAddData::new();
        let web_socket_actor_handlers = Vec::new();

        Self {
            receiver,
            baseline,
            web_socket_actor_handlers,
        }
    }

    #[instrument]
    async fn handle_change(&mut self, change: FileChangeData) {
        let FileChangeData { removed, added } = &change;

        tracing::info!("known files changed: {}", json!(change));

        let mut new_baseline =
            Vec::with_capacity(self.baseline.0.len() + added.0.len() - removed.0.len());

        let (mut idx_baseline, mut index_added) = (0usize, 0usize);
        let (baseline_len, added_len) = (self.baseline.0.len(), added.0.len());
        while idx_baseline < baseline_len || index_added < added_len {
            if idx_baseline < baseline_len && removed.0.contains(&self.baseline.0[idx_baseline].0) {
                idx_baseline += 1;
            } else {
                if index_added < added_len {
                    if idx_baseline < baseline_len
                        && self.baseline.0[idx_baseline].1 > added.0[index_added].1
                    {
                        new_baseline.push(self.baseline.0[idx_baseline].clone());
                        idx_baseline += 1;
                    } else {
                        new_baseline.push(added.0[index_added].clone());
                        index_added += 1;
                    }
                } else {
                    new_baseline.push(self.baseline.0[idx_baseline].clone());
                    idx_baseline += 1;
                }
            }
        }

        let mut new_handlers = Vec::new();

        for handler in self.web_socket_actor_handlers.drain(..) {
            let result = handler.send_change(change.clone()).await;
            match result {
                Ok(_) => {
                    new_handlers.push(handler);
                }
                Err(_) => {
                    handler
                        .join_handle()
                        .await
                        .expect("Expected handle to be joinable");
                }
            }
        }

        self.web_socket_actor_handlers = new_handlers;

        tracing::debug!("new baseline: {:?}", &new_baseline);

        self.baseline = FileAddData(new_baseline);
    }

    #[instrument]
    async fn run(mut self) {
        tracing::debug!("actor started");
        while let Some(msg) = self.receiver.recv().await {
            match msg {
                FileTrackerActorEvent::Change(change) => {
                    self.handle_change(change).await;
                }
                FileTrackerActorEvent::AddWebSocketActorHandler(handler) => {
                    tracing::debug!("adding web socket actor handler");
                    let result = handler
                        .send_change(FileChangeData {
                            removed: FileRemoveData(Vec::new()),
                            added: self.baseline.clone(),
                        })
                        .await;
                    match result {
                        Ok(_) => {
                            self.web_socket_actor_handlers.push(handler);
                        }
                        Err(_) => {
                            handler
                                .join_handle()
                                .await
                                .expect("Expected handle to be joinable");
                        }
                    }
                }
            }
        }
        tracing::debug!("actor stopped");
    }
}

#[derive(Clone, Debug)]
pub struct FileTrackerActorHandler {
    sender: mpsc::Sender<FileTrackerActorEvent>,
}

impl FileTrackerActorHandler {
    pub async fn new(
        shutdown_actor_handler: &ShutdownActorHandler,
    ) -> crate::error::Result<Arc<Self>> {
        let (tx, rx) = mpsc::channel::<FileTrackerActorEvent>(8);
        let actor = FileTrackerActor::new(rx);
        let join_handle = tokio::spawn(actor.run());
        shutdown_actor_handler.add_join_handle(join_handle).await?;

        let result = Arc::from(Self { sender: tx });
        shutdown_actor_handler.add_droppable(result.clone()).await?;
        Ok(result)
    }

    pub async fn send_change(&self, change: FileChangeData) -> crate::error::Result<()> {
        self.sender
            .send(FileTrackerActorEvent::Change(change))
            .await?;
        Ok(())
    }

    pub async fn add_web_socket_actor_handler(
        &self,
        handler: WebSocketActorHandler,
    ) -> crate::error::Result<()> {
        self.sender
            .send(FileTrackerActorEvent::AddWebSocketActorHandler(handler))
            .await?;
        Ok(())
    }
}
