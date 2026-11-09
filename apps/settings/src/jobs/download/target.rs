//! 要下的一样东西：哪个仓库的哪个 Release，下完怎么装。

use std::path::{Path, PathBuf};

use super::{Install, Item, Repo};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub item: Item,

    pub repo: Repo,

    /// Release 标签，`<名字>-v主.次.修订`。
    pub tag: String,

    pub install: Install,
}

impl Target {
    /// 下载用的临时目录，放在装到的目录旁边（同一个盘，装的时候改名就行）；点开头，扫词库的不会看见。
    pub(super) fn work_dir(&self, downloads: &Path) -> PathBuf {
        self.install
            .parent(downloads)
            .join(format!(".download-{}", self.tag))
    }
}
