use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum NeuralError {
    #[error("io error reading {path}: {source}")]
    Io {
        /// 出错的文件。
        path: PathBuf,

        /// 底层错误。
        source: std::io::Error,
    },

    #[error("bad json in {path}: {source}")]
    Json {
        /// 出错的文件。
        path: PathBuf,

        /// 底层错误。
        source: serde_json::Error,
    },

    #[error("model error: {0}")]
    Candle(#[from] candle_core::Error),

    #[error("corrupt model: {0}")]
    Corrupt(&'static str),
}
