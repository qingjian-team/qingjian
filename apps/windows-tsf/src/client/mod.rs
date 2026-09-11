//! DLL 的「引擎层」：不跑 Engine，而是连独立 Server 进程，把按键 / 上屏编排成协议消息。
//!
//! [`EngineClient`] 平台无关（泛型在任意 [`Read`](std::io::Read) + [`Write`](std::io::Write) 双工流上），
//! 可脱离 Windows 端到端测；`cfg(windows)` 的 [`pipe`] 负责真正连 `\\.\pipe\qingjian`。

mod engine;
mod response;

#[cfg(windows)]
pub mod pipe;

pub use engine::EngineClient;
pub use response::KeyResponse;
