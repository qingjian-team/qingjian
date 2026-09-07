use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum GlossError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("api error: {0}")]
    Api(#[from] async_openai::error::OpenAIError),

    #[error("request timed out after {0} s")]
    Timeout(u64),

    #[error("empty reply")]
    EmptyReply,

    #[error("no words selected from {0}")]
    NoWords(PathBuf),
}
