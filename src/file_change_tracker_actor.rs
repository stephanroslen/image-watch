use crate::file_tracker_actor::{FileTrackerActor, FileTrackerActorEvent};
use std::cmp::Reverse;
use std::{collections::HashSet, mem::take, path::PathBuf, time::Duration};
use tokio::{
    sync::mpsc,
    task::spawn_blocking,
    time::{Interval, MissedTickBehavior},
};
use tracing::instrument;

#[derive(Debug)]
pub struct FileChangeTrackerActor {
    file_tracker_actor_sender: mpsc::Sender<FileTrackerActorEvent>,
    rescrape_timer: Interval,
    path_prefix: PathBuf,
    file_extensions: HashSet<String>,
    known_files: HashSet<PathBuf>,
}

impl FileChangeTrackerActor {
    pub fn new(
        file_tracker_actor_sender: mpsc::Sender<FileTrackerActorEvent>,
        rescrape_interval: Duration,
        path_prefix: PathBuf,
        file_extensions: Vec<String>,
    ) -> Self {
        let file_extensions = file_extensions.into_iter().collect();
        let mut rescrape_timer = tokio::time::interval(rescrape_interval);
        // continue with intended interval even if the timer is missed
        rescrape_timer.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let known_files = HashSet::new();

        Self {
            file_tracker_actor_sender,
            rescrape_timer,
            path_prefix,
            file_extensions,
            known_files,
        }
    }

    #[instrument(level = "trace")]
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
            FileTrackerActor::send_change(&self.file_tracker_actor_sender, file_change_data)
                .await?;
        }

        self.known_files = found;

        Ok(())
    }

    #[instrument(level = "trace")]
    pub async fn run(mut self, mut receiver: mpsc::Receiver<()>) {
        loop {
            tokio::select! {
                msg = receiver.recv() => match msg {
                    Some(_) => {},
                    None => break,
                },
                _ = self.rescrape_timer.tick() => {
                    self.rescrape().await.expect("Expected rescrape to succeed");
                }
            }
        }
    }
}
