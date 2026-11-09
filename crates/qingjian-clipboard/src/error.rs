//! 存储的错误。

#[derive(Debug, thiserror::Error)]
pub enum ClipboardError {
    #[error("config file is not valid TOML: {0}")]
    BadConfig(String),

    #[error("clipboard database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("cannot create clipboard data directory: {0}")]
    Directory(#[from] std::io::Error),
}
