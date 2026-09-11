//! 青简 Windows 输入法的 TSF 文本服务 DLL。
//!
//! TSF 会把这个 DLL 加载进每一个应用进程，所以这里不放 Engine（那在独立的 Server 进程）。DLL 只做两件事：
//! 把 TSF 的按键翻译成协议消息发给 Server、把 Server 回的 [`Frame`](qingjian_platform::protocol::Frame) 画成候选窗口。
//!
//! - [`client`]：引擎层，连 Server 的管道客户端。平台无关部分泛型在任意双工字节流上，可脱离 Windows 端到端测。
//! - `com`（`cfg(windows)`）：COM 入口、TSF 接口实现、编辑会话 / 组句、候选窗口、云联想轮询、自注册。

pub mod client;
#[cfg(windows)]
mod com;
pub mod error;

pub use error::ClientError;
