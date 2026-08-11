use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("audio decode error: {0}")]
    AudioDecode(String),

    #[error("record not found: {0}")]
    NotFound(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("resource not ready: {0}")]
    NotReady(String),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl Error {
    pub fn app_data_dir() -> Result<PathBuf> {
        dirs::data_local_dir()
            .map(|base| base.join("atcs-soundboard"))
            .ok_or_else(|| Error::NotReady("cannot resolve app data directory".into()))
    }

    pub fn ensure_data_dir() -> Result<PathBuf> {
        let dir = Self::app_data_dir()?;
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
