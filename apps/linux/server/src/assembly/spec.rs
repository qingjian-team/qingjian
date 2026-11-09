//! Linux Engine 装配所需的产品数据与用户路径。
use std::path::PathBuf;

use qingjian_core::Language;
use qingjian_platform::{DictionariesConfig, UserDirs};

use super::LanguageModelFiles;

/// 装配要用的数据文件。除词库外都可选：缺哪个就少哪个功能。
pub struct AssemblySpec {
    /// 主词库（`.qj` 或 TSV）。
    pub dict: PathBuf,

    /// 学习语言的释义表。
    pub glossary: Option<(Language, PathBuf)>,

    /// 英→中释义表（英文候选的中文释义）。
    pub english_glossary: Option<PathBuf>,

    /// 英文词表。
    pub english: Option<PathBuf>,

    /// 英文音标表（随包 `english-ipa.qj`）。可选，没有就只是不显示音标。
    pub english_ipa: Option<PathBuf>,

    /// emoji 表（多张合成一张）。
    pub emoji: Vec<PathBuf>,

    /// 语言模型；没有就退化成一元词频整句。
    pub language_model: Option<LanguageModelFiles>,

    /// 随包领域词库目录。
    pub bundled_dicts_dir: Option<PathBuf>,

    /// `[dictionaries]` 配置。
    pub dictionaries: DictionariesConfig,

    /// 用户数据（XDG data home 下的 `qingjian/`，分类见 `UserDirs`）；没有就都只在内存。
    pub user: Option<UserDirs>,

    /// 是否写输入日志（`[general] input_log`）。
    pub input_log: bool,
}

impl AssemblySpec {
    pub fn new(dict: impl Into<PathBuf>) -> Self {
        Self {
            dict: dict.into(),
            glossary: None,
            english_glossary: None,
            english: None,
            english_ipa: None,
            emoji: Vec::new(),
            language_model: None,
            bundled_dicts_dir: None,
            dictionaries: DictionariesConfig::default(),
            user: None,
            input_log: false,
        }
    }
}
