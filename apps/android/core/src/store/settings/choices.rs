//! 键盘设置页列的选项与缺省值，壳缓存着用。

use serde::Serialize;

use super::Choice;

#[derive(Debug, Clone, Serialize)]
pub struct SettingChoices {
    /// 模糊音规则，按 PC 设置程序的顺序。
    pub fuzzy: Vec<Choice>,

    /// 学习语言，最后一项是不显示译文。
    pub learning_languages: Vec<Choice>,

    pub default_learning_language: &'static str,

    /// 功能面板「繁体」开着时的字形，不含简体。
    pub traditional_variants: Vec<Choice>,

    pub default_traditional_variant: &'static str,

    /// 缺省打开的随包领域词库（文件名不含扩展名）。
    pub default_domains: Vec<&'static str>,

    /// 剪贴板最多存多少条的几档。
    pub clipboard_limits: Vec<usize>,

    pub default_clipboard_limit: usize,
}
