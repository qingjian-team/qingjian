//! 生词本页的一个词条。

use serde::Serialize;

/// 一个词条：译词、带它上屏的中文与那一句、几个次数、最近见到的日期、认识没有、还算不算生词、等级。
#[derive(Debug, Serialize)]
pub struct VocabularyWord {
    pub word: String,

    pub source: String,

    pub example: String,

    /// 在候选旁看到的次数。
    pub seen: u32,

    pub committed: u32,

    /// 直接打出这个外文词的次数。
    pub used: u32,

    /// `2026-10-19`。
    pub last: String,

    pub known: bool,

    /// 候选里还标成生词。
    pub fresh: bool,

    /// 等级表里的级别（`A1`、`N5`）；没有等级表或表里没有为空串。
    pub level: String,
}
