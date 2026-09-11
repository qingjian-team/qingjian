//! 青简 Windows 输入法 Server 的库部分：Engine 装配（[`assembly`]）与协议分派（[`Router`]），
//! 供 bin `qingjian-server` 与集成测试（`tests/`）共用。
//!
//! 平台适配层（TSF DLL）与传输层（命名管道）不在这里，见 crate 根 `README.md`。

pub mod assembly;
pub mod dispatch;
pub mod error;
pub mod ipc;
mod session;

pub use dispatch::Router;
pub use error::ServerError;
