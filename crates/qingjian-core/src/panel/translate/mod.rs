//! 面板的「翻译」组：框里攒着用户打的原文，停一会儿才去翻译，回来的译文由壳显示在应用的 preedit 里。
//! 目标语言就是这一组的分类（[`super::SymbolPanel`] 里 `[` `]` 换）；原文是外文时照「翻译选中文字」的判断改译回中文。

mod cache;
mod draft;
mod job;
mod status;

pub(crate) use cache::TranslationCache;
pub(crate) use draft::TranslateDraft;
pub(crate) use job::TranslateJob;
pub use status::TranslateStatus;

use crate::Language;

/// 翻译组列出的目标语言，按这个顺序；与学习语言的选项一致。
pub(crate) const TARGET_LANGUAGES: [Language; 5] = [
    Language::English,
    Language::Japanese,
    Language::Spanish,
    Language::Vietnamese,
    Language::German,
];
