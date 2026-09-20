use std::path::PathBuf;

use qingjian_core::Language;
use qingjian_platform::{AuxCodeConfig, DictionariesConfig};

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

    /// emoji 表（多张合成一张）。
    pub emoji: Vec<PathBuf>,

    /// 语言模型；没有就退化成一元词频整句。
    pub language_model: Option<LanguageModelFiles>,

    /// 随包领域词库目录。
    pub bundled_dicts_dir: Option<PathBuf>,

    /// `[dictionaries]` 配置。
    pub dictionaries: DictionariesConfig,

    /// 随包辅码码表目录（随包根 `codes/`）。
    pub bundled_codes_dir: Option<PathBuf>,

    /// `[aux_code]` 配置。
    pub aux_code: AuxCodeConfig,

    /// 词汇等级表目录（`levels-<语言>.tsv`）。
    pub levels_dir: Option<PathBuf>,

    /// 用户数据目录（`%APPDATA%\Qingjian`）；没有就都只在内存。
    pub user_dir: Option<PathBuf>,

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
            emoji: Vec::new(),
            language_model: None,
            bundled_dicts_dir: None,
            dictionaries: DictionariesConfig::default(),
            bundled_codes_dir: None,
            aux_code: AuxCodeConfig::default(),
            levels_dir: None,
            user_dir: None,
            input_log: false,
        }
    }
}
