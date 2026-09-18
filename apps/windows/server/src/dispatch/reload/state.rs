//! 配置热加载记的状态。

use std::path::PathBuf;
use std::time::{Instant, SystemTime};

use qingjian_core::Language;
use qingjian_dictionary::Dictionary;
use qingjian_platform::{AuxCodeConfig, DictionariesConfig, code_tables, extra_dictionaries};
use qingjian_predict::PredictConfig;

/// 随包与用户数据目录：启动与热加载用的是同一批（词库、码表）。
/// 分开传参数会越传越长，且热加载与原路径不一致时找不到文件。
#[derive(Debug, Clone, Default)]
pub struct DataDirs {
    /// 用户数据根目录（个人释义表在它下面；`dicts/` / `codes/` 的父目录）。
    pub user_root: Option<PathBuf>,

    /// 随包领域词库目录。
    pub bundled_dicts: Option<PathBuf>,

    /// 随包辅码码表目录（随包根 `codes/`）。
    pub bundled_codes: Option<PathBuf>,

    /// 用户导入词库目录（`<用户目录>/dicts`）。
    pub user_dicts: Option<PathBuf>,

    /// 用户导入码表目录（`<用户目录>/codes`）。
    pub user_codes: Option<PathBuf>,
}

impl DataDirs {
    /// 用户 `codes/` 的逐文件快照（路径、mtime、长度）；没配目录为空。启动与热加载轮询共用。
    pub(super) fn code_snapshot(&self) -> Vec<(PathBuf, Option<SystemTime>, u64)> {
        self.user_codes
            .as_deref()
            .map(code_tables::snapshot)
            .unwrap_or_default()
    }

    /// 用户 `dicts/` 的同一份快照；没配目录为空。
    pub(super) fn dict_snapshot(&self) -> Vec<(PathBuf, Option<SystemTime>, u64)> {
        self.user_dicts
            .as_deref()
            .map(extra_dictionaries::snapshot)
            .unwrap_or_default()
    }
}

/// 热加载状态。
pub(crate) struct ConfigReload {
    /// `config.toml` 路径。
    pub(super) config_path: PathBuf,

    /// 上次看文件的时间（节流用）。
    pub(super) last_check: Instant,

    /// 随包数据根目录（释义表在 `data/generated` 下）。
    pub(super) root: PathBuf,

    /// 随包与用户数据目录。
    pub(super) dirs: DataDirs,

    /// 上次看到的码表文件快照（逐文件 mtime + 长度）：设置页刚导入一张表、或手工拷进半个
    /// `.qj` 后补全，都靠它即时生效。
    pub(super) code_files: Vec<(PathBuf, Option<SystemTime>, u64)>,

    /// 上次看到的 mtime。
    pub(super) last_mtime: Option<SystemTime>,

    /// 已应用的 `[predict]`。
    pub(super) applied_predict: PredictConfig,

    /// 已应用的 `[dictionaries]`。
    pub(super) applied_dictionaries: DictionariesConfig,

    /// 已应用的 `[aux_code]`。
    pub(super) applied_aux_code: AuxCodeConfig,

    /// 已应用的学习语言（`None` 为关）。
    pub(super) applied_language: Option<Language>,

    /// 最近加载的用户词库文件快照（路径、修改时间、长度）。
    pub(super) dictionary_files: Vec<(PathBuf, Option<SystemTime>, u64)>,
}

impl ConfigReload {
    /// 按上次有效配置装配词库，不把用户目录中的学习数据当作词库。
    pub(super) fn load_dictionaries(&self) -> Vec<Dictionary> {
        let dictionaries = extra_dictionaries::load(
            self.dirs.bundled_dicts.as_deref(),
            self.dirs.user_dicts.as_deref(),
            &self.applied_dictionaries,
        );
        tracing::info!(count = dictionaries.len(), "附加词库已热重装");
        dictionaries
    }
}
