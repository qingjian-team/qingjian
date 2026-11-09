//! 配置热加载记的状态。

use std::path::PathBuf;
use std::time::{Instant, SystemTime};

use qingjian_core::Language;
use qingjian_dictionary::Dictionary;
use qingjian_platform::{DictionariesConfig, LocalModelConfig, UserDirs, extra_dictionaries};
use qingjian_predict::PredictConfig;
use qingjian_translate::DOWNLOADED_DIR;

/// 热加载状态。
pub(crate) struct ConfigReload {
    /// `config.toml` 路径。
    pub(super) config_path: PathBuf,

    /// 上次看文件的时间（节流用）。
    pub(super) last_check: Instant,

    /// 上次看到的 mtime。
    pub(super) last_mtime: Option<SystemTime>,

    /// 随包数据根目录（释义表、随包领域词库在它下面）。
    pub(super) root: PathBuf,

    /// 用户数据（分类见 `UserDirs`）。
    pub(super) user: UserDirs,

    /// 上次看到的附加词库（导入的与下载的）快照（路径、mtime、长度）。
    pub(super) dictionary_files: Vec<(PathBuf, Option<SystemTime>, u64)>,

    /// 上次看到的 `downloads/glossaries/`（设置页下载的释义表）快照：装上新表不改配置，靠它发现并重装。
    pub(super) glossary_files: Vec<(PathBuf, Option<SystemTime>, u64)>,

    /// 已应用的 `[dictionaries]`。
    pub(super) applied_dictionaries: DictionariesConfig,

    /// 已应用的 `[predict]`。
    pub(super) applied_predict: PredictConfig,

    /// 已应用的 `[model]`。
    pub(super) applied_model: LocalModelConfig,

    /// 已应用的学习语言（`None` 为关）。
    pub(super) applied_language: Option<Language>,
}

impl ConfigReload {
    /// 附加词库（导入的与下载的）的逐文件快照。
    pub(super) fn dict_snapshot(&self) -> Vec<(PathBuf, Option<SystemTime>, u64)> {
        self.user
            .extra_dict_dirs()
            .iter()
            .flat_map(|dir| extra_dictionaries::snapshot(dir))
            .collect()
    }

    /// `downloads/glossaries/` 的逐文件快照。
    pub(super) fn glossary_snapshot(&self) -> Vec<(PathBuf, Option<SystemTime>, u64)> {
        extra_dictionaries::snapshot(&self.user.downloads().join(DOWNLOADED_DIR))
    }

    /// 按已应用的 `[dictionaries]` 装配附加词库，目录与启动装配同款。
    pub(super) fn load_dictionaries(&self) -> Vec<Dictionary> {
        let user_dicts = self.user.extra_dict_dirs();
        let user_dicts: Vec<&std::path::Path> = user_dicts.iter().map(PathBuf::as_path).collect();
        let dictionaries = extra_dictionaries::load(
            Some(&self.root.join("data/generated/dicts")),
            &user_dicts,
            &self.applied_dictionaries,
        );
        tracing::info!(count = dictionaries.len(), "附加词库已热重装");
        dictionaries
    }
}
