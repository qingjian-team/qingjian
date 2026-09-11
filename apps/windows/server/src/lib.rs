//! 青简 Windows 输入法 Server 的库部分：Engine 装配（[`assembly`]）、协议分派（[`Router`]）与传输（[`ipc`]），
//! 供 bin `qingjian-server` 与集成测试共用。

pub mod assembly;
pub mod dispatch;
pub mod error;
pub mod ipc;

pub use assembly::{AssemblySpec, LanguageModelFiles};
pub use dispatch::{Router, RouterConfig};
pub use error::ServerError;
