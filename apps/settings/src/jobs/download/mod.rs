//! 后台下载：「实验功能」的模型与「词库」页的词库都走这里。在自己的线程里下，进度与结果放在共享的槽里，有变化就叫醒窗口。
//! 一次只下一个、别的排队；关掉设置窗口下载也就停了（排着的也丢掉），下次从头下。

mod downloads;
mod fetch;
mod install;
mod item;
mod manifest;
mod model;
mod repo;
mod shared;
mod stage;
mod target;

pub use downloads::Downloads;
pub use fetch::fetch_small;
pub use install::Install;
pub use item::Item;
pub use model::Pack;
pub use repo::Repo;
pub use stage::Stage;
pub use target::Target;

/// 下载的东西装在哪：下面的 `dicts/`、`glossaries/`、`models/` 与输入法读的同一处。
fn downloads_dir() -> Option<std::path::PathBuf> {
    crate::system::paths::user_dirs().map(|dirs| dirs.downloads())
}
