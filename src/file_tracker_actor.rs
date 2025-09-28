use crate::authentication::{
    Token, authentication_token_store_actor::AuthenticationTokenStoreActorEvent,
};
use crate::web_socket_actor::WebSocketActorEvent;
use crate::{
    error::Result,
    file_change_data::{FileAddData, FileChangeData, FileRemoveData},
    web_socket_actor::WebSocketActor,
};
use axum::extract::ws::WebSocket;
use std::mem::take;
use tokio::{sync::mpsc, task::spawn_blocking};
use tracing::instrument;

#[derive(Debug)]
struct WebSocketActorSenderAndJoinHandle {
    sender: mpsc::Sender<WebSocketActorEvent>,
    join_handle: tokio::task::JoinHandle<()>,
}

impl WebSocketActorSenderAndJoinHandle {
    fn extract_join_handle(self) -> tokio::task::JoinHandle<()> {
        self.join_handle
    }
}

#[derive(Debug)]
pub enum FileTrackerActorEvent {
    Change(FileChangeData),
    AddWebSocket(WebSocket, Token),
}

#[derive(Debug)]
pub struct FileTrackerActor {
    baseline: FileAddData,
    web_socket_actor_senders_and_join_handles: Vec<WebSocketActorSenderAndJoinHandle>,
    authentication_token_store_actor_sender: mpsc::Sender<AuthenticationTokenStoreActorEvent>,
    token_refresh_interval: std::time::Duration,
}

impl FileTrackerActor {
    pub fn new(
        authentication_token_store_actor_sender: mpsc::Sender<AuthenticationTokenStoreActorEvent>,
        token_refresh_interval: std::time::Duration,
    ) -> Self {
        let baseline = FileAddData::new();
        let web_socket_actor_senders_and_join_handles = Vec::new();

        Self {
            baseline,
            web_socket_actor_senders_and_join_handles,
            authentication_token_store_actor_sender,
            token_refresh_interval,
        }
    }

    #[instrument(level = "trace")]
    async fn handle_change(&mut self, change: FileChangeData) {
        tracing::info!("known files changed: {:?}", &change);

        {
            let mut survivors = Vec::new();

            for sender_and_join_handle in self.web_socket_actor_senders_and_join_handles.drain(..) {
                let result =
                    WebSocketActor::send_change(&sender_and_join_handle.sender, change.clone())
                        .await;
                match result {
                    Ok(_) => {
                        survivors.push(sender_and_join_handle);
                    }
                    Err(_) => {
                        sender_and_join_handle
                            .extract_join_handle()
                            .await
                            .expect("Expected handle to be joinable");
                    }
                }
            }

            self.web_socket_actor_senders_and_join_handles = survivors;
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

    #[instrument(level = "trace")]
    pub async fn run(mut self, mut receiver: mpsc::Receiver<FileTrackerActorEvent>) {
        while let Some(msg) = receiver.recv().await {
            match msg {
                FileTrackerActorEvent::Change(change) => {
                    self.handle_change(change).await;
                }
                FileTrackerActorEvent::AddWebSocket(ws, token) => {
                    let (sender, receiver) = mpsc::channel::<_>(8);
                    let ws_actor = WebSocketActor::new(
                        ws,
                        self.authentication_token_store_actor_sender.clone(),
                        self.token_refresh_interval,
                        token,
                    );
                    let join_handle = tokio::task::spawn(ws_actor.run(receiver));
                    let sender_and_join_handle = WebSocketActorSenderAndJoinHandle {
                        sender,
                        join_handle,
                    };
                    let result = WebSocketActor::send_change(
                        &sender_and_join_handle.sender,
                        FileChangeData {
                            removed: FileRemoveData(Vec::new()),
                            added: self.baseline.clone(),
                        },
                    )
                    .await;
                    match result {
                        Ok(_) => {
                            self.web_socket_actor_senders_and_join_handles
                                .push(sender_and_join_handle);
                        }
                        Err(_) => {
                            sender_and_join_handle
                                .extract_join_handle()
                                .await
                                .expect("Expected handle to be joinable");
                        }
                    }
                }
            }
        }

        self.shutdown_web_socket_actor_handlers().await;
    }

    async fn shutdown_web_socket_actor_handlers(mut self) {
        for sender_and_join_handle in self.web_socket_actor_senders_and_join_handles.drain(..) {
            let join_handle = sender_and_join_handle.extract_join_handle();
            join_handle
                .await
                .expect("Expected web socket actor to be joinable");
        }
    }

    pub async fn send_change(
        sender: &mpsc::Sender<FileTrackerActorEvent>,
        change: FileChangeData,
    ) -> Result<()> {
        sender.send(FileTrackerActorEvent::Change(change)).await?;
        Ok(())
    }

    pub async fn add_web_socket(
        sender: &mpsc::Sender<FileTrackerActorEvent>,
        ws: WebSocket,
        token: Token,
    ) -> Result<()> {
        sender
            .send(FileTrackerActorEvent::AddWebSocket(ws, token))
            .await?;
        Ok(())
    }
}
