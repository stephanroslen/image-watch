use crate::{
    error::Result,
    file_change_data::{FileAddData, FileChangeData, FileRemoveData},
    web_socket_actor::WebSocketActorHandler,
};
use std::{mem::take, sync::Arc};
use tokio::{
    sync::mpsc,
    task::{JoinSet, spawn_blocking},
};
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
        tracing::info!("known files changed: {:?}", &change);

        {
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
        }

        let baseline = take(&mut self.baseline);

        let new_baseline = spawn_blocking(move || {
            let FileChangeData { removed, added } = &change;

            let mut new_baseline =
                Vec::with_capacity(baseline.0.len() + added.0.len() - removed.0.len());

            let (mut idx_baseline, mut index_added) = (0usize, 0usize);
            let (baseline_len, added_len) = (baseline.0.len(), added.0.len());
            while idx_baseline < baseline_len || index_added < added_len {
                if idx_baseline < baseline_len && removed.0.contains(&baseline.0[idx_baseline].0) {
                    idx_baseline += 1;
                } else if index_added < added_len {
                    if idx_baseline < baseline_len
                        && baseline.0[idx_baseline].1 > added.0[index_added].1
                    {
                        new_baseline.push(baseline.0[idx_baseline].clone());
                        idx_baseline += 1;
                    } else {
                        new_baseline.push(added.0[index_added].clone());
                        index_added += 1;
                    }
                } else {
                    new_baseline.push(baseline.0[idx_baseline].clone());
                    idx_baseline += 1;
                }
            }
            new_baseline
        })
        .await
        .expect("Expected task to complete");

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
    pub fn new(join_set: &mut JoinSet<()>) -> Result<Arc<Self>> {
        let (tx, rx) = mpsc::channel::<FileTrackerActorEvent>(8);
        let actor = FileTrackerActor::new(rx);
        join_set.spawn(actor.run());

        let result = Arc::from(Self { sender: tx });
        Ok(result)
    }

    pub async fn send_change(&self, change: FileChangeData) -> Result<()> {
        self.sender
            .send(FileTrackerActorEvent::Change(change))
            .await?;
        Ok(())
    }

    pub async fn add_web_socket_actor_handler(&self, handler: WebSocketActorHandler) -> Result<()> {
        self.sender
            .send(FileTrackerActorEvent::AddWebSocketActorHandler(handler))
            .await?;
        Ok(())
    }
}
