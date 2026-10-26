//! 冷启动实验错误：失败必须中断，不能静默跳过难例。

#[derive(Debug, thiserror::Error)]
pub enum ColdError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Neural(#[from] qingjian_neural::NeuralError),

    #[error("invalid cold evaluation row: {0}")]
    Invalid(String),

    #[error("Hanzhang Tongbian model requires a <sep> token in its vocabulary")]
    Model,
}
