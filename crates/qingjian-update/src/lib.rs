//! 检查更新：读官网的版本索引 `releases.json`，验 ed25519 签名，按平台、CPU 与渠道挑出比当前新的版本。
//!
//! 只提示不安装。壳在自己的定时器里调 [`Checker::poll`]，到点了它起一个一次性线程去查，结果写进数据目录的 `update.json`；
//! 菜单与设置页用 [`Checker::available`] 取要提示的版本。设计与发版侧的签名见 `docs/design/update.md`。

#![forbid(unsafe_code)]

mod checker;
mod error;
mod index;
mod target;
mod version;

pub use checker::{Available, Checker, UpdateState};
pub use error::UpdateError;
pub use index::{Asset, Index, PUBLIC_KEYS, Release, SCHEMA_VERSION, verify};
pub use target::Target;
pub use version::Version;

/// 下载页，提示里点开的就是它。
pub const DOWNLOAD_URL: &str = "https://qingjian.app/download";
