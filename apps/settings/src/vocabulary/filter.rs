//! 生词本页顶上的筛选。

use jiff::civil::Date;
use qingjian_core::FRESH_UNTIL;
use qingjian_learning::VocabularyEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Filter {
    #[default]
    All,
    /// 候选里还标着生词的。
    Fresh,
    /// 本周第一次见到的。
    ThisWeek,
    /// 标了认识的。
    Known,
}

impl Filter {
    pub const ALL: [Self; 4] = [Self::All, Self::Fresh, Self::ThisWeek, Self::Known];

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "全部",
            Self::Fresh => "生词",
            Self::ThisWeek => "本周新见",
            Self::Known => "认识了",
        }
    }

    pub fn keeps(self, entry: &VocabularyEntry, week_start: Date) -> bool {
        match self {
            Self::All => true,
            Self::Fresh => !entry.known && entry.seen < FRESH_UNTIL,
            Self::ThisWeek => entry.first >= week_start,
            Self::Known => entry.known,
        }
    }
}
