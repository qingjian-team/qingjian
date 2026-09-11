//! 青简 Windows 输入法的 **TSF 文本服务 DLL**。
//!
//! TSF 会把这个 DLL 加载进每一个应用进程，所以这里**不放** [`qingjian_core::Engine`]（那在独立的
//! Server 进程，见 `qingjian-windows`）。DLL 只做两件事：把 TSF 的按键翻译成协议消息发给 Server、
//! 把 Server 回的 [`Frame`](qingjian_platform::protocol::Frame) 画成候选窗口。
//!
//! 分层：
//! - [`client`]：「引擎层」——连 Server 的管道客户端与按键/上屏编排。平台无关部分（[`client::EngineClient`]）
//!   泛型在任意双工字节流上，可脱离 Windows 端到端测；真正连 `\\.\pipe\qingjian` 的部分在 `cfg(windows)`。
//! - `com`（`cfg(windows)`，C 步接）：最小 COM 链路（`DllGetClassObject` → `IClassFactory` →
//!   `ITfTextInputProcessor` → `ITfKeyEventSink`）与 `DllRegisterServer` 注册。

pub mod client;
#[cfg(windows)]
mod com;
pub mod error;

pub use error::ClientError;
