//! Server ↔ DLL 的 IPC 协议类型。
//!
//! Windows 的 TSF DLL 会被加载进每一个应用进程，所以 [`qingjian_core::Engine`] 不能待在 DLL 里，
//! 得跑在独立的 Server 进程；DLL 只做 IPC，把系统按键翻译成 [`ClientMessage`] 发给 Server，
//! 把 Server 回的 [`ServerMessage`] 画到候选窗口。结构与 Weasel（WeaselServer）/ 水杉（Server 进程）一致。
//!
//! 这里的类型两端共用，必须可序列化（serde）。协议只描述「按键进、要画什么出」这一层，
//! 不复制 Core 的排序 / 词库逻辑：候选直接用 [`qingjian_core::CandidateList`]，preedit 分段用
//! [`PreeditSegment`]（Core 内部的 `MarkedSegment` 的可序列化镜像，避免协议耦合 Core 的内部枚举）。

mod client;
mod server;
mod session;

pub mod frame;
pub mod key;

pub use client::ClientMessage;
pub use frame::{Frame, PreeditKind, PreeditSegment};
pub use key::{KeyEvent, KeyOutcome};
pub use server::ServerMessage;
pub use session::SessionId;
