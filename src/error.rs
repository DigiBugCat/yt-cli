use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("API key not set. Run `yt-cli init` to configure.")]
    ApiKeyMissing,

    #[error("Download failed: {0}")]
    Download(String),

    #[error("Transcription failed: {0}")]
    Transcription(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
