//! 仓库根目录的清单 `index.json`（词库仓库与模型仓库各一份）：设置窗口打开时取一次，Gitee 与 GitHub 同时取、先回来的用；
//! 取到的存进缓存目录，下次没取到时用它兜底。

mod job;
mod state;
mod status;

pub use job::RemoteIndex;
pub use state::IndexState;
pub use status::Status;

use crate::jobs::download::Repo;

/// 清单里的一条：哪个仓库的清单、缓存成什么文件、怎么读。
pub trait Listing: Clone + Send + 'static {
    /// 从哪个仓库取。
    const REPO: Repo;

    /// 缓存文件名，放在缓存目录里。
    const CACHE_FILE: &'static str;

    /// 日志里叫它什么。
    const WHAT: &'static str;

    /// 读整份 `index.json`，只留这一版输入法列得出的。
    fn parse(bytes: &[u8]) -> Result<Vec<Self>, String>;

    fn id(&self) -> &str;

    /// 从没取到过、也没有缓存时列什么。
    fn fallback() -> Vec<Self> {
        Vec::new()
    }
}
