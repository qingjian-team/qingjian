//! 组句中上屏用的两个键（`[shortcut] commit_candidate` / `commit_raw`）：写法与解析在 [`CommitKey`]，
//! 一次按键该走哪条在 [`CommitKeys::route`]，Windows 与 Linux 的 Server 共用。

mod key;
mod route;

pub use key::CommitKey;
pub use route::{CommitKeys, CommitRoute};
