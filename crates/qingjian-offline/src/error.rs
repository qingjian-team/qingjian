//! 本地翻译错误；不包含用户原文。

#[derive(Debug, thiserror::Error)]
pub enum OfflineError {
    #[error("inference: {0}")]
    Inference(String),

    #[error("{0}")]
    Invalid(&'static str),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
