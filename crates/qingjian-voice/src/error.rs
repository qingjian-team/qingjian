//! 识别出错。

use thiserror::Error;

#[derive(Debug, Error)]
pub enum VoiceError {
    #[error("not configured: missing API key")]
    MissingApiKey,

    #[error("connect failed: {0}")]
    Connect(String),

    #[error("rejected by server: HTTP {0}")]
    Rejected(u16),

    #[error("server error {code}: {message}")]
    Server { code: u32, message: String },

    #[error("connection closed before the final result")]
    Closed,

    #[error("no final result within {0} ms")]
    Timeout(u64),

    #[error("malformed frame: {0}")]
    Protocol(&'static str),

    #[error("websocket: {0}")]
    Socket(#[from] tungstenite::Error),
}

impl VoiceError {
    /// 给用户看的一句话（`#[error]` 的文案给日志用）。
    pub fn describe(&self) -> String {
        match self {
            Self::MissingApiKey => "还没填语音识别的密钥".to_owned(),
            Self::Connect(_) | Self::Socket(_) | Self::Closed => {
                "连不上语音识别服务，检查网络".to_owned()
            }
            Self::Rejected(401 | 403) => "语音识别的密钥无效或没有开通这项服务".to_owned(),
            Self::Rejected(status) => format!("语音识别服务拒绝了请求（{status}）"),
            Self::Server {
                code: 45_000_002, ..
            } => "没听到声音".to_owned(),
            Self::Server {
                code: 55_000_031, ..
            } => "语音识别服务繁忙，稍后再试".to_owned(),
            Self::Server { code, .. } => format!("语音识别出错（{code}）"),
            Self::Timeout(_) => "语音识别没有回应，检查网络".to_owned(),
            Self::Protocol(_) => "语音识别服务的回复看不懂".to_owned(),
        }
    }
}
