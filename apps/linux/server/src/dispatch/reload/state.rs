//! 配置热加载记的状态。

use std::path::PathBuf;
use std::time::{Instant, SystemTime};

use qingjian_core::Language;
use qingjian_dictionary::Dictionary;
use qingjian_platform::code_tables::AuxCodeSources;
use qingjian_platform::{
    Config, DictionariesConfig, LocalModelConfig, UserDirs, code_schemes, extra_dictionaries,
};
use qingjian_predict::PredictConfig;
use qingjian_translate::DOWNLOADED_DIR;

/// 热加载状态。
pub(crate) struct ConfigReload {
    /// 上次成功应用的配置；方案文件单独变化时沿用，不读取尚未写完的配置。
    pub(super) applied_config: Config,

    /// 方案描述、码表和导入形码词库的路径、修改时间与长度。
    pub(super) scheme_files: Vec<(PathBuf, Option<SystemTime>, u64)>,

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

    pub(super) fn scheme_snapshot(&self) -> Vec<(PathBuf, Option<SystemTime>, u64)> {
        let mut files = Vec::new();
        for directory in [self.user.custom(), self.root.join("data/generated")] {
            files.extend(
                AuxCodeSources {
                    user_dir: Some(directory),
                    ..Default::default()
                }
                .snapshot(),
            );
        }
        files.extend(code_schemes::words_snapshot(&code_schemes::user_words_dir(
            &self.user.custom(),
        )));
        let legacy = self.user.custom().join("wubi/wubi86.tsv");
        if let Ok(metadata) = std::fs::metadata(&legacy) {
            files.push((legacy, metadata.modified().ok(), metadata.len()));
        }
        if let Some(id) = self.applied_config.general.code()
            && let Some(entry) = code_schemes::find(
                &id,
                Some(&self.user.custom()),
                Some(&self.root.join("data/generated/schemes")),
            )
            && let Ok(metadata) = std::fs::metadata(&entry.table)
        {
            files.push((entry.table, metadata.modified().ok(), metadata.len()));
        }
        files.sort();
        files.dedup();
        files
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
