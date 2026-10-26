//! Linux Server：持有 Core Engine，通过 Unix domain socket 服务多个 Fcitx5 会话。
pub mod assembly;
pub mod dispatch;
pub mod error;
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub mod ipc;
pub use assembly::{AssemblySpec, LanguageModelFiles};
pub use dispatch::{Router, RouterConfig, find_model};
pub use error::ServerError;
