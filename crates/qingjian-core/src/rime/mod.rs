//! 可选 librime 后端：用户安装运行库与方案，青简只接收原始候选和预编辑文本。
//! 所有 C API 调用由同一把锁串行化，运行库活到最后一个会话销毁之后。

mod api;
mod candidate;
mod commit;
mod context;
mod error;
mod library;
mod menu;
mod options;
mod runtime;
mod session;
mod snapshot;
mod traits;

pub use candidate::RimeCandidate;
pub use error::RimeError;
pub use menu::RimeMenu;
pub use options::RimeOptions;
pub(crate) use session::Session;
