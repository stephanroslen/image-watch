use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Axum error: {0}")]
    AxumError(#[from] axum::Error),
    #[error("DotEnvy error: {0}")]
    DotEnvyError(#[from] dotenvy::Error),
    #[error("SerdeJson error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("std::var::EnvError: {0}")]
    StdVarEnvError(#[from] std::env::VarError),
    #[error("std::io::Error: {0}")]
    StdIoError(#[from] std::io::Error),
    #[error("std::num::ParseBoolError: {0}")]
    StdParseBoolError(#[from] std::str::ParseBoolError),
    #[error("std::num::ParseIntError: {0}")]
    StdNumParseIntError(#[from] std::num::ParseIntError),
    #[error("std::sync::PoisonError: {0}")]
    StdSyncPoisonError(String),
    #[error("tokio::sync::mpsc::error::SendError: {0}")]
    TokioSyncMpscSendError(String),
    #[error("tokio::task::JoinError: {0}")]
    TokioJoinError(#[from] tokio::task::JoinError),
    #[error("tokio::sync::oneshot::error::RecvError: {0}")]
    TokioSyncOneshotReceiveError(#[from] tokio::sync::oneshot::error::RecvError),
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(err: std::sync::PoisonError<T>) -> Self {
        Error::StdSyncPoisonError(err.to_string())
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for Error {
    fn from(err: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Error::TokioSyncMpscSendError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
