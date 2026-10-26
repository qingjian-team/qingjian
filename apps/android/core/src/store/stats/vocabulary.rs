//! 统计详情页的词汇一节。

use qingjian_core::VocabularySummary;
use serde::Serialize;

use super::StatsLevel;

/// 学习语言的词汇汇总，有等级表时按级分。
#[derive(Debug, Serialize)]
pub struct StatsVocabulary {
    /// 学习语言的代码（`en`）。
    pub language: String,

    pub seen: u64,

    pub familiar: u64,

    pub committed: u64,

    pub used: u64,

    pub new_this_week: u64,

    pub levels: Vec<StatsLevel>,
}

impl StatsVocabulary {
    pub fn new(language: &str, summary: VocabularySummary) -> Self {
        Self {
            language: language.to_owned(),
            seen: summary.seen,
            familiar: summary.familiar,
            committed: summary.committed,
            used: summary.used,
            new_this_week: summary.new_this_week,
            levels: summary.levels.into_iter().map(StatsLevel::from).collect(),
        }
    }
}
