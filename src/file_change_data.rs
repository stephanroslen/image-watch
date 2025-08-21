use serde::Serialize;
use serde_with::{TimestampMilliSeconds, serde_as};
use std::{path::PathBuf, time::SystemTime};

#[serde_as]
#[derive(Clone, Debug, Serialize)]
pub struct FileAddData(
    #[serde_as(as = "Vec<(_, TimestampMilliSeconds<i64>)>")] pub Vec<(PathBuf, SystemTime)>,
);

impl FileAddData {
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

#[serde_as]
#[derive(Clone, Debug, Serialize)]
pub struct FileRemoveData(pub Vec<PathBuf>);

#[derive(Clone, Debug, Serialize)]
pub struct FileChangeData {
    pub removed: FileRemoveData,
    pub added: FileAddData,
}

impl FileChangeData {
    pub fn new(removed: Vec<PathBuf>, added: Vec<(PathBuf, SystemTime)>) -> Self {
        Self {
            removed: FileRemoveData(removed),
            added: FileAddData(added),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.removed.0.is_empty() && self.added.0.is_empty()
    }

    pub fn is_not_empty(&self) -> bool {
        !self.is_empty()
    }
}
