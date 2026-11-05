//! 词库仓库的清单 `index.json`：设置窗口打开时取一次，Gitee 与 GitHub 同时取、先回来的用；
//! 取到的存进数据目录的 `dict-index.json`，下次没取到时用它兜底。

mod entry;
mod index_file;
mod job;
mod state;
mod status;

pub use entry::DictEntry;
pub use job::DictIndex;
pub use state::IndexState;
pub use status::Status;
