//! 用户目录的分类布局，三个桌面平台的输入法与设置程序共用：东西按「谁产生的、从哪来的」放进各自的文件夹，根目录只留配置。
//!
//! ```text
//! <根>/config.toml        配置（Linux 按 XDG 放 ~/.config/qingjian/，不在这里）
//! <根>/learning/          输入法从打字里学到的：词频、造词、选词、降权、个人 n-gram、敲错表、英文词、个人释义表
//! <根>/records/           统计与记录：输入统计、生词本、面板用量、AI 用量、输入日志
//! <根>/custom/            用户自己导入、自己做的：dicts/ code_dicts/ codes/ schemes/ themes/
//! <根>/clipboard/         青简剪贴板
//! <本机>/downloads/       从青简仓库下载的：dicts/ glossaries/ models/
//! <本机>/cache/           删了会自己重建的：词库列表缓存、更新状态
//! ```
//!
//! 「本机」在 Windows 上是 `%LOCALAPPDATA%\Qingjian`：`%APPDATA%` 跟着域账户漫游同步，几百 MB 的模型和缓存不该跟着跑；
//! mac 与 Linux 没有这个区分，和根是同一处。运行日志照各系统的惯例另放，不在这里。
//! 旧版本平铺在根目录的文件由 [`UserDirs::prepare`] 搬进来，见 `migrate.rs`。

mod migrate;

use std::path::{Path, PathBuf};

/// 一个用户的数据目录：`root` 放用户产生的，`local` 放下载与缓存。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserDirs {
    root: PathBuf,

    local: PathBuf,
}

impl UserDirs {
    pub fn new(root: PathBuf, local: PathBuf) -> Self {
        Self { root, local }
    }

    /// 下载与缓存也放在根下（mac、Linux）。
    pub fn at(root: PathBuf) -> Self {
        Self {
            local: root.clone(),
            root,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 学习数据：`FrequencyLearner`、个人释义表等都在这一层里写各自的文件。
    pub fn learning(&self) -> PathBuf {
        self.root.join("learning")
    }

    /// 统计与记录：输入统计、生词本、面板用量、AI 用量。
    pub fn records(&self) -> PathBuf {
        self.root.join("records")
    }

    /// 输入日志（开了才写，每次上屏一行）。
    pub fn input_log(&self) -> PathBuf {
        self.records().join("input-log.jsonl")
    }

    /// 从青简仓库下载的东西的根：下面是 `dicts/`、`glossaries/`、`models/`，设置程序的下载按这个相对位置装。
    pub fn downloads(&self) -> PathBuf {
        self.local.join("downloads")
    }

    /// 下载的附加词库。
    pub fn downloaded_dicts(&self) -> PathBuf {
        self.downloads().join("dicts")
    }

    /// 下载的本地模型，一个模型一个子目录。
    pub fn models(&self) -> PathBuf {
        self.downloads().join("models")
    }

    /// 用户自己导入、自己做的东西的根：形码方案与五笔词库按 `code_schemes` 的相对位置放在这下面。
    pub fn custom(&self) -> PathBuf {
        self.root.join("custom")
    }

    /// 导入的拼音词库；移除的挪进它下面的 `removed/`。
    pub fn custom_dicts(&self) -> PathBuf {
        self.custom().join("dicts")
    }

    /// 附加词库从哪两处加载：导入的与下载的，同一组开关。
    pub fn extra_dict_dirs(&self) -> [PathBuf; 2] {
        [self.custom_dicts(), self.downloaded_dicts()]
    }

    /// 导入的辅码码表。
    pub fn code_tables(&self) -> PathBuf {
        self.custom().join("codes")
    }

    /// 自定义主题，一个主题一个子目录。
    pub fn themes(&self) -> PathBuf {
        self.custom().join("themes")
    }

    /// 升级后要补下的词库（词库仓库的 id，一行一个）：老版本随包的领域词库、通用扩展词库和几张释义表新版改成下载，
    /// 老用户开着的由设置程序依次下回来、下好一个划掉一个。新装的在 [`Self::prepare`] 里写成空文件；
    /// 文件不在说明是从老版本升上来、还没算过要下哪些。
    pub fn restore_list(&self) -> PathBuf {
        self.downloads().join("restore.txt")
    }

    /// 删了会自己重建的文件。
    pub fn cache(&self) -> PathBuf {
        self.local.join("cache")
    }

    /// 青简剪贴板的历史与同步数据。
    pub fn clipboard(&self) -> PathBuf {
        self.root.join("clipboard")
    }

    /// 建好各分类目录，把旧版本平铺在根目录的文件搬进来。输入法与设置程序启动时各调一次；
    /// 两边同时跑也没事，搬过的不会再搬，出错只记日志。
    pub fn prepare(&self) {
        let fresh =
            std::fs::read_dir(&self.root).map_or(true, |mut entries| entries.next().is_none());
        for dir in [
            self.learning(),
            self.records(),
            self.custom(),
            self.downloads(),
            self.cache(),
        ] {
            if let Err(error) = std::fs::create_dir_all(&dir) {
                tracing::warn!(dir = %dir.display(), %error, "user data folder not created");
            }
        }
        migrate::run(self);
        if fresh && let Err(error) = std::fs::write(self.restore_list(), "") {
            tracing::warn!(%error, "restore list not written");
        }
    }
}

#[cfg(test)]
mod tests;
