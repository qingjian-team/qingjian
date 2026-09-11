//! 青简 Windows 输入法的 TSF 文本服务 DLL。
//!
//! TSF 会把这个 DLL 加载进每一个应用进程，所以 Engine 不在这里（在独立的 Server 进程）。
//! DLL 只把 TSF 的按键翻成协议消息发给 Server、把回的文本 / 拼音行写进文档；候选窗口由 Server 自绘。
//!
//! - [`client`]：引擎层，连 Server 的管道客户端，平台无关部分可脱离 Windows 端到端测。
//! - `com`（`cfg(windows)`）：COM 入口、TSF 接口实现、编辑会话 / 组句、云联想轮询、自注册。

pub mod client;
#[cfg(windows)]
mod com;
pub mod error;

pub use error::ClientError;
