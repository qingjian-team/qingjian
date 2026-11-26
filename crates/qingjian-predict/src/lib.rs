//! 云联想：[`qingjian_core::Predictor`] 的网络实现，接 OpenAI 兼容的聊天接口（首个目标是 DeepSeek）。
//!
//! 请求在独立线程里发，主线程只往通道里丢请求、从通道里取结果，永远不阻塞输入。
//! 「最新请求优先」：防抖窗口内只发最后一个请求；结果带序号，Core 丢弃过期的。
//!
//! [`CloudGlossFiller`] 是释义兜底（[`qingjian_core::GlossFiller`]）的网络实现，独立线程，攒批问、不丢请求。

#![forbid(unsafe_code)]

// 两个 TLS 特性同时开时（工作区一起编：桌面缺省 native-tls，安卓开 rustls）用的是系统 TLS，
// 根证书表这时编不进去，这里引一下免得 Cargo 报它没用
#[cfg(feature = "rustls")]
use webpki_roots as _;

mod cache;
mod chat_client;
mod cloud_predictor;
mod config;
mod connection;
mod error;
mod gloss;
mod prompt;
mod reply;
mod suspension;
mod usage;
mod worker;

pub use cloud_predictor::CloudPredictor;
pub use config::PredictConfig;
pub use connection::{ConnectionReport, ConnectionTest};
pub use error::PredictError;
pub use gloss::CloudGlossFiller;

pub use usage::{
    API_USAGE_FILE, API_USAGE_HOURLY_FILE, ApiUsageHour, ApiUsageHours, ApiUsageStats,
};
