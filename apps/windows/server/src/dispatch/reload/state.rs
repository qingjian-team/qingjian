//! 配置热加载记的状态。

use std::path::PathBuf;
use std::time::{Instant, SystemTime};

use qingjian_core::Language;
use qingjian_dictionary::Dictionary;
use qingjian_platform::code_schemes;
use qingjian_platform::code_tables::AuxCodeSources;
use qingjian_platform::{
    AuxCodeConfig, DictionariesConfig, UpdateConfig, UserDirs, extra_dictionaries,
};
use qingjian_predict::PredictConfig;
use qingjian_translate::DOWNLOADED_DIR;

/// 随包与用户数据目录：启动与热加载用的是同一批（词库、码表）。
/// 分开传参数会越传越长，且热加载与原路径不一致时找不到文件。
#[derive(Debug, Clone, Default)]
pub struct DataDirs {
    /// 用户数据（分类见 `UserDirs`）；没有就都只在内存。
    pub user: Option<UserDirs>,

    /// 随包领域词库目录。
    pub bundled_dicts: Option<PathBuf>,

    /// 随包辅码码表目录（随包根 `data/generated/codes/`，与 `dicts/` 并列）。
    pub bundled_codes: Option<PathBuf>,

    /// 随包方案目录（辅码方案在这里与用户的 `custom\schemes\` 里找）。
    pub bundled_schemes: Option<PathBuf>,
}

impl DataDirs {
    /// 辅码码表从哪几处找。
    pub(super) fn aux_sources(&self) -> AuxCodeSources {
        AuxCodeSources {
            bundled_codes: self.bundled_codes.clone(),
            user_codes: self.user.as_ref().map(UserDirs::code_tables),
            bundled_schemes: self.bundled_schemes.clone(),
            user_dir: self.user.as_ref().map(UserDirs::custom),
        }
    }

    /// 学习数据目录（个人释义表在这里）。
    pub(super) fn learning(&self) -> Option<PathBuf> {
        self.user.as_ref().map(UserDirs::learning)
    }

    /// 下载目录（释义表在它的 `glossaries\` 里）。
    pub(super) fn downloads(&self) -> Option<PathBuf> {
        self.user.as_ref().map(UserDirs::downloads)
    }

    /// 附加词库：导入的与下载的。
    fn extra_dict_dirs(&self) -> Vec<PathBuf> {
        self.user
            .as_ref()
            .map(UserDirs::extra_dict_dirs)
            .into_iter()
            .flatten()
            .collect()
    }

    /// 用户 `codes/` 与 `schemes/` 的逐文件快照（路径、mtime、长度）。启动与热加载轮询共用。
    pub(super) fn code_snapshot(&self) -> Vec<(PathBuf, Option<SystemTime>, u64)> {
        self.aux_sources().snapshot()
    }

    /// 用户 `custom\code_dicts\`（导入的五笔词库）的同一份快照；没有用户目录为空。
    pub(super) fn code_word_snapshot(&self) -> Vec<(PathBuf, Option<SystemTime>, u64)> {
        self.user
            .as_ref()
            .map(|user| code_schemes::words_snapshot(&code_schemes::user_words_dir(&user.custom())))
            .unwrap_or_default()
    }

    /// `downloads\glossaries\`（设置页下载的释义表）的同一份快照；没有用户目录为空。
    pub(super) fn glossary_snapshot(&self) -> Vec<(PathBuf, Option<SystemTime>, u64)> {
        self.downloads()
            .map(|dir| extra_dictionaries::snapshot(&dir.join(DOWNLOADED_DIR)))
            .unwrap_or_default()
    }

    /// 附加词库（导入的与下载的）的同一份快照；没有用户目录为空。
    pub(super) fn dict_snapshot(&self) -> Vec<(PathBuf, Option<SystemTime>, u64)> {
        self.extra_dict_dirs()
            .iter()
            .flat_map(|dir| extra_dictionaries::snapshot(dir))
            .collect()
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

    /// 上次看到的形码词库快照：变了就按当前配置重装形码方案。
    pub(super) code_word_files: Vec<(PathBuf, Option<SystemTime>, u64)>,

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

    /// 最近看到的下载释义表快照：设置页装上新表不改配置，靠它发现并重装。
    pub(super) glossary_files: Vec<(PathBuf, Option<SystemTime>, u64)>,

    /// 当前的 `[update]`。
    pub(super) update: UpdateConfig,

    /// 检查更新：结果写进 `cache\update.json`，设置程序的「关于」页读它；拿不到用户目录时没有。
    pub(super) updates: Option<qingjian_update::Checker>,
}

impl ConfigReload {
    /// 按上次有效配置装配词库，不把用户目录中的学习数据当作词库。
    pub(super) fn load_dictionaries(&self) -> Vec<Dictionary> {
        let user_dicts = self.dirs.extra_dict_dirs();
        let user_dicts: Vec<&std::path::Path> = user_dicts.iter().map(PathBuf::as_path).collect();
        let dictionaries = extra_dictionaries::load(
            self.dirs.bundled_dicts.as_deref(),
            &user_dicts,
            &self.applied_dictionaries,
        );
        tracing::info!(count = dictionaries.len(), "附加词库已热重装");
        dictionaries
    }
}
