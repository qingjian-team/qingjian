//! 配置热加载记的状态。

use std::path::PathBuf;
use std::time::SystemTime;

use qingjian_platform::DictionariesConfig;
use qingjian_predict::PredictConfig;

/// 热加载状态。
pub(crate) struct ConfigReload {
    /// `config.toml` 路径。
    pub(super) config_path: PathBuf,

    /// 随包领域词库目录。
    pub(super) bundled_dicts_dir: Option<PathBuf>,

    /// 用户数据目录（导入词库在其 `dicts/` 下）。
    pub(super) user_dir: Option<PathBuf>,

    /// 上次看到的 mtime。
    pub(super) last_mtime: Option<SystemTime>,

    /// 已应用的 `[predict]`。
    pub(super) applied_predict: PredictConfig,

    /// 已应用的 `[dictionaries]`。
    pub(super) applied_dictionaries: DictionariesConfig,
}
