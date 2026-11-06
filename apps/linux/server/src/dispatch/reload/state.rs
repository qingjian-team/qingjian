//! 配置热加载记的状态。

use std::path::PathBuf;
use std::time::{Instant, SystemTime};

use qingjian_core::Language;
use qingjian_dictionary::Dictionary;
use qingjian_platform::code_tables::AuxCodeSources;
use qingjian_platform::{
    Config, DictionariesConfig, LocalModelConfig, code_schemes, extra_dictionaries,
};
use qingjian_predict::PredictConfig;

/// 热加载状态。
pub(crate) struct ConfigReload {
    pub(super) applied_config: Config,

    pub(super) scheme_files: Vec<(PathBuf, Option<SystemTime>, u64)>,

    /// `config.toml` 路径。
    pub(super) config_path: PathBuf,

    /// 上次看文件的时间（节流用）。
    pub(super) last_check: Instant,

    /// 上次看到的 mtime。
    pub(super) last_mtime: Option<SystemTime>,

    /// 随包数据根目录（释义表、随包领域词库在它下面）。
    pub(super) root: PathBuf,

    /// 用户数据目录（个人释义表、`dicts/` 在它下面）。
    pub(super) user_dir: PathBuf,

    /// 上次看到的用户 `dicts/` 快照（路径、mtime、长度）。
    pub(super) dictionary_files: Vec<(PathBuf, Option<SystemTime>, u64)>,

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
    pub(super) fn scheme_snapshot(&self) -> Vec<(PathBuf, Option<SystemTime>, u64)> {
        let mut files = Vec::new();
        for directory in [&self.user_dir, &self.root.join("assets")] {
            files.extend(
                AuxCodeSources {
                    user_dir: Some(directory.clone()),
                    ..Default::default()
                }
                .snapshot(),
            );
        }
        files.extend(code_schemes::words_snapshot(&code_schemes::user_words_dir(
            &self.user_dir,
        )));
        let legacy = self.user_dir.join("wubi/wubi86.tsv");
        if let Ok(metadata) = std::fs::metadata(&legacy) {
            files.push((legacy, metadata.modified().ok(), metadata.len()));
        }
        if let Some(id) = self.applied_config.general.code()
            && let Some(entry) = code_schemes::find(
                &id,
                Some(&self.user_dir),
                Some(&self.root.join("assets/schemes")),
            )
            && let Ok(metadata) = std::fs::metadata(&entry.table)
        {
            files.push((entry.table, metadata.modified().ok(), metadata.len()));
        }
        files.sort();
        files.dedup();
        files
    }

    fn user_dicts(&self) -> PathBuf {
        self.user_dir.join("dicts")
    }

    /// 用户 `dicts/` 的逐文件快照。
    pub(super) fn dict_snapshot(&self) -> Vec<(PathBuf, Option<SystemTime>, u64)> {
        extra_dictionaries::snapshot(&self.user_dicts())
    }

    /// 按已应用的 `[dictionaries]` 装配附加词库，目录与启动装配同款。
    pub(super) fn load_dictionaries(&self) -> Vec<Dictionary> {
        let dictionaries = extra_dictionaries::load(
            Some(&self.root.join("data/generated/dicts")),
            Some(&self.user_dicts()),
            &self.applied_dictionaries,
        );
        tracing::info!(count = dictionaries.len(), "附加词库已热重装");
        dictionaries
    }
}
