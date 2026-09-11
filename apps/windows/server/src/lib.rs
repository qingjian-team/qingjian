//! 青简 Windows 输入法 Server 的库部分：Engine 装配（[`assembly`]）、协议分派（[`Router`]）与传输（[`ipc`]），
//! 供 bin `qingjian-server` 与集成测试共用。

pub mod assembly;
pub mod dispatch;
pub mod error;
pub mod ipc;
/// 候选窗口自绘线程（Server 进程内画候选，才能盖过微软商店 / 任务栏搜索这些高 z-band 宿主）；仅 Windows。
#[cfg(windows)]
pub mod ui;

pub use assembly::{AssemblySpec, LanguageModelFiles};
pub use dispatch::{Router, RouterConfig};
pub use error::ServerError;
