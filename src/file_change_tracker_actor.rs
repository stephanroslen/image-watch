use crate::file_tracker_actor::FileTrackerActorHandler;
use crate::shutdown_actor::ShutdownActorHandler;
use std::cmp::Reverse;
use std::{
    collections::HashSet,
    mem::take,
    path::PathBuf,
    sync::{Arc, Weak},
    time::Duration,
};
use tokio::{sync::mpsc, task::spawn_blocking, time::Interval};
use tracing::instrument;

#[derive(Debug)]
enum FileChangeTrackerActorEvent {}

#[derive(Debug)]
struct FileChangeTrackerActor {
    receiver: mpsc::Receiver<FileChangeTrackerActorEvent>,
    file_tracker_actor_handler: Weak<FileTrackerActorHandler>,
    rescrape_timer: Interval,
    path_prefix: PathBuf,
    file_extensions: HashSet<String>,
    known_files: HashSet<PathBuf>,
}

impl FileChangeTrackerActor {
    fn new(
        receiver: mpsc::Receiver<FileChangeTrackerActorEvent>,
        file_tracker_actor_handler: Weak<FileTrackerActorHandler>,
        rescrape_interval: Duration,
        path_prefix: PathBuf,
        file_extensions: Vec<String>,
    ) -> Self {
        let file_extensions = file_extensions.into_iter().collect();
        let rescrape_timer = tokio::time::interval(rescrape_interval);
        let known_files = HashSet::new();

        Self {
            receiver,
            file_tracker_actor_handler,
            rescrape_timer,
            path_prefix,
            file_extensions,
            known_files,
        }
    }

    #[instrument]
    async fn rescrape(&mut self) -> crate::error::Result<()> {
        let known_files = take(&mut self.known_files);
        let path_prefix = self.path_prefix.clone();
        let file_extensions = self.file_extensions.clone();

        let (found, file_change_data) = spawn_blocking(move || {
            let found: HashSet<_> = walkdir::WalkDir::new(&path_prefix)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .filter_map(|e| {
                    e.path()
                        .strip_prefix(&path_prefix)
                        .map(|p| p.to_path_buf())
                        .ok()
                })
                .filter_map(|e| {
                    let extension = e.extension()?.to_str()?;
                    if file_extensions.contains(extension) {
                        Some(e)
                    } else {
                        None
                    }
                })
                .collect();

            let file_change_data = crate::file_change_data::FileChangeData::new(
                known_files.difference(&found).cloned().collect(),
                {
                    let mut tmp: Vec<_> = found
                        .difference(&known_files)
                        .cloned()
                        .filter_map(|path| {
                            path_prefix
                                .join(path.clone())
                                .metadata()
                                .ok()
                                .and_then(|metadata| metadata.modified().ok())
                                .map(|timestamp| (path, timestamp))
                        })
                        .collect();
                    tmp.sort_by_key(|(_, time)| Reverse(*time));
                    tmp
                },
            );

            (found, file_change_data)
        })
        .await?;

        if file_change_data.is_not_empty() {
            tracing::debug!("file change data: {:?}", &file_change_data);
            if let Some(file_tracker_actor_handler) = self.file_tracker_actor_handler.upgrade() {
                file_tracker_actor_handler
                    .send_change(file_change_data)
                    .await?;
            }
        }

        self.known_files = found;

        Ok(())
    }

    #[instrument]
    async fn run(mut self) {
        tracing::debug!("actor started");
        loop {
            tokio::select! {
                msg = self.receiver.recv() => match msg {
                    Some(_) => {},
                    None => break,
                },
                _ = self.rescrape_timer.tick() => {
                    self.rescrape().await.expect("Expected rescrape to succeed");
                }
            }
        }
        tracing::debug!("actor stopped");
    }
}

#[derive(Clone, Debug)]
pub struct FileChangeTrackerActorHandler {
    _sender: mpsc::Sender<FileChangeTrackerActorEvent>,
}

impl FileChangeTrackerActorHandler {
    pub async fn new(
        shutdown_actor_handler: &ShutdownActorHandler,
        file_tracker_actor_handler: Weak<FileTrackerActorHandler>,
        rescrape_interval: Duration,
        path_prefix: PathBuf,
        file_extensions: Vec<String>,
    ) -> crate::error::Result<Arc<Self>> {
        let (tx, rx) = mpsc::channel::<FileChangeTrackerActorEvent>(8);
        let actor = FileChangeTrackerActor::new(
            rx,
            file_tracker_actor_handler,
            rescrape_interval,
            path_prefix,
            file_extensions,
        );
        shutdown_actor_handler
            .add_join_handle(tokio::spawn(actor.run()))
            .await?;

        let result = Arc::from(Self { _sender: tx });
        shutdown_actor_handler.add_droppable(result.clone()).await?;
        Ok(result)
    }
}
