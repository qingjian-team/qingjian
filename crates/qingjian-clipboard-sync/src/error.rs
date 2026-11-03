//! 同步过程的错误。

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("connection failed: {0}")]
    Io(#[from] std::io::Error),

    /// 配对码不对，或对方被冒充，握手都走到这里。
    #[error("handshake failed")]
    Handshake,

    #[error("peer is not paired")]
    UnknownPeer,

    /// 对方已经取消了和这台的配对。
    #[error("peer has unpaired this device")]
    NotPaired,

    #[error("protocol violation: {0}")]
    Protocol(String),

    #[error("sync storage failed: {0}")]
    Storage(String),
}

impl From<snow::Error> for SyncError {
    fn from(_: snow::Error) -> Self {
        Self::Handshake
    }
}

impl From<serde_json::Error> for SyncError {
    fn from(error: serde_json::Error) -> Self {
        Self::Protocol(error.to_string())
    }
}
