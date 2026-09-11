use std::path::PathBuf;

use qingjian_core::Language;
use qingjian_platform::DictionariesConfig;

use super::LanguageModelFiles;

/// 装配要用的数据文件。随包数据（词库 / 释义表 / 等级表 / 语言模型 …）与用户数据目录分开给。
/// 除词库外都可选：缺哪个就少哪个功能，与 macOS 的 `host::init` 一致。
pub struct AssemblySpec {
    /// 主词库（`.qj` 或 TSV）。
    pub dict: PathBuf,

    /// 学习语言的释义表。
    pub glossary: Option<(Language, PathBuf)>,

    /// 英文候选的中文释义表（英→中）。
    pub english_glossary: Option<PathBuf>,

    /// 英文词表（英文模式候选）。
    pub english: Option<PathBuf>,

    /// emoji 表（中文表 + 英文表合成一张）。
    pub emoji: Vec<PathBuf>,

    /// 语言模型；没有就退化成一元词频整句。
    pub language_model: Option<LanguageModelFiles>,

    /// 随包领域词库目录，按 `[dictionaries] domains` 挑。
    pub bundled_dicts_dir: Option<PathBuf>,

    /// 附加词库配置（`[dictionaries]`）。
    pub dictionaries: DictionariesConfig,

    /// 词汇等级表目录（`levels-<语言>.tsv`），「统计」按级数词汇用。
    pub levels_dir: Option<PathBuf>,

    /// 用户数据目录（`%APPDATA%\Qingjian`）：学习数据、输入日志、统计、词汇记录、个人释义表、
    /// 用户导入的词库 `dicts/` 都在这里。没有（本机测试）就都只在内存。
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
            levels_dir: None,
            user_dir: None,
            input_log: false,
        }
    }
}
