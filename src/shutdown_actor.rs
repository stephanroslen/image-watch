use std::{fmt::Debug, sync::Arc};
use tokio::{sync::mpsc, task::JoinHandle};
use tracing::instrument;

#[derive(Debug)]
enum ShutdownActorEvent {
    AddJoinHandle(JoinHandle<()>),
    AddDroppable(Arc<dyn Send + Sync + Debug>),
    Shutdown,
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
                            panic!("Expected channel not to close before shutdown event!");
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
                                ShutdownActorEvent::Shutdown => {
                                    for droppable in self.droppables.drain(..) {
                                        tracing::debug!("Dropping {:?}", droppable);
                                    }
                                    for join_handle in self.join_handles.drain(..) {
                                        tracing::debug!("Joining {:?}", join_handle);
                                        join_handle.await.expect("Expected join handle to complete");
                                        tracing::debug!("Joined");
                                    }
                                    break;
                                }
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
    own_join_handle: JoinHandle<()>,
}

impl ShutdownActorHandler {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<ShutdownActorEvent>(8);
        let actor = ShutdownActor::new(rx);

        let own_join_handle = tokio::spawn(actor.run());

        Self {
            sender: tx,
            own_join_handle,
        }
    }

    #[instrument]
    pub async fn shutdown(self) {
        self.sender
            .send(ShutdownActorEvent::Shutdown)
            .await
            .expect("Expected to be able to send shutdown event to shutdown actor");
        self.own_join_handle
            .await
            .expect("Expected to be able to join shutdown actor");
        tracing::debug!("Shutdown actor stopped");
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
