//! 生词本页顶上的筛选（同 PC 生词本的「筛选」）。

use jiff::civil::Date;
use qingjian_core::FRESH_UNTIL;
use qingjian_learning::VocabularyEntry;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VocabularyFilter {
    #[default]
    All,

    /// 候选里还标着生词的。
    Fresh,

    /// 最近 7 天第一次见到的（与汇总的「近 7 天新见」同一个数）。
    Week,

    /// 标了认识的。
    Known,
}

impl VocabularyFilter {
    pub fn keeps(self, entry: &VocabularyEntry, recent_from: Date) -> bool {
        match self {
            Self::All => true,
            Self::Fresh => is_fresh(entry),
            Self::Week => entry.first >= recent_from,
            Self::Known => entry.known,
        }
    }
}

/// 候选里还标成生词：没标认识、见的次数还不够。
pub fn is_fresh(entry: &VocabularyEntry) -> bool {
    !entry.known && entry.seen < FRESH_UNTIL
}
