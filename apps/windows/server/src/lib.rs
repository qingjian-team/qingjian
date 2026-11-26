//! Server 的库部分：Engine 装配（[`assembly`]）、协议分派（[`Router`]）与传输（[`ipc`]），供 bin 与集成测试共用。

pub mod assembly;
pub mod clipboard_process;
pub mod dispatch;
pub mod error;
pub mod ipc;
/// 英文候选朗读（Windows SAPI）；其他平台是空实现。
pub mod speech;
/// 候选窗口 / 状态条的自绘线程；仅 Windows。
#[cfg(windows)]
pub mod ui;

pub use assembly::{AssemblySpec, LanguageModelFiles};
pub use dispatch::{Router, RouterConfig};
pub use error::ServerError;
