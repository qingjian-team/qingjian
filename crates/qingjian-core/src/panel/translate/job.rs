//! 一次翻译要做的事。

use crate::Language;

/// 原文与实际的目标语言（外文原文是中文）；请求、译文、缓存都按它对账。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct TranslateJob {
    pub(crate) source: String,

    pub(crate) language: Language,
}
