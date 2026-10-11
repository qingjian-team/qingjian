//! 当前整句候选的异步译文；只在对应的组句与学习语言下使用。

use crate::Language;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentenceTranslation {
    /// 被翻译的中文候选，必须与请求里的原文完全一致。
    pub source: String,

    /// 当前学习语言。
    pub language: Language,

    /// 可直接上屏的整句译文。
    pub text: String,
}
