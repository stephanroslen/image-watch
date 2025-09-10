use std::{fmt::Debug, sync::Arc};
use tokio::task::JoinSet;
use tokio::{sync::mpsc, task::JoinHandle};
use tracing::instrument;

#[derive(Debug)]
enum ShutdownActorEvent {
    AddJoinHandle(JoinHandle<()>),
    AddDroppable(Arc<dyn Send + Sync + Debug>),
}

#[derive(Debug)]
struct ShutdownActor {
    receiver: mpsc::Receiver<ShutdownActorEvent>,
    join_handles: Vec<JoinHandle<()>>,
    droppables: Vec<Arc<dyn Send + Sync + Debug>>,
}

impl ShutdownActor {
    fn new(receiver: mpsc::Receiver<ShutdownActorEvent>) -> Self {
        let join_handles = Vec::new();
        let droppables = Vec::new();
        Self {
            receiver,
            join_handles,
            droppables,
        }
    }

    #[instrument]
    async fn run(mut self) {
        tracing::debug!("actor started");
        loop {
            tokio::select! {
                msg = self.receiver.recv() => {
                    match msg {
                        None => {
                            for droppable in self.droppables.drain(..) {
                                tracing::debug!("Dropping {:?}", droppable);
                            }
                            for join_handle in self.join_handles.drain(..) {
                                tracing::debug!("Joining {:?}", join_handle);
                                join_handle.await.expect("Expected join handle to complete");
                                tracing::debug!("Joined");
                            }
                            break;
                        },
                        Some(msg) => {
                            match msg {
                                ShutdownActorEvent::AddJoinHandle(join_handle) => {
                                    tracing::debug!("Added join handle {:?}", &join_handle);
                                    self.join_handles.push(join_handle);
                                },
                                ShutdownActorEvent::AddDroppable(droppable) => {
                                    tracing::debug!("Added droppable {:?}", &droppable);
                                    self.droppables.push(droppable);
                                },
                            }
                        }
                    }
                }
            }
        }
        tracing::debug!("actor stopped");
    }
}

#[derive(Debug)]
pub struct ShutdownActorHandler {
    sender: mpsc::Sender<ShutdownActorEvent>,
}

impl ShutdownActorHandler {
    pub fn new(join_set: &mut JoinSet<()>) -> Self {
        let (tx, rx) = mpsc::channel::<ShutdownActorEvent>(8);
        let actor = ShutdownActor::new(rx);

        join_set.spawn(actor.run());

        Self { sender: tx }
    }

    pub async fn add_join_handle(&self, join_handle: JoinHandle<()>) -> crate::error::Result<()> {
        self.sender
            .send(ShutdownActorEvent::AddJoinHandle(join_handle))
            .await?;
        Ok(())
    }

    pub async fn add_droppable(
        &self,
        droppable: Arc<dyn Send + Sync + Debug>,
    ) -> crate::error::Result<()> {
        self.sender
            .send(ShutdownActorEvent::AddDroppable(droppable))
            .await?;
        Ok(())
    }
}
