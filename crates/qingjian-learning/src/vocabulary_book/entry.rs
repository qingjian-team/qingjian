//! 生词本里一个译词的记录：看到、上屏、直接打出的次数，首见末见，用户标的「认识了」，最近一次的中文与例句。

use jiff::civil::Date;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VocabularyEntry {
    /// 上屏时在候选窗口里的轮次。
    pub seen: u32,

    /// 上屏过带它的候选几次。
    pub committed: u32,

    /// 直接打出过几次。
    pub used: u32,

    /// 第一次记录的日期。
    pub first: Date,

    /// 最近一次记录的日期。
    pub last: Date,

    /// 用户在生词本里标了认识：不再标生词。
    pub known: bool,

    /// 最近一次带它上屏的中文候选；还没记过为空。
    pub source: String,

    /// 那次上屏所在的整句；还没记过为空。
    pub example: String,
}

impl VocabularyEntry {
    pub(super) fn new(date: Date) -> Self {
        Self {
            seen: 0,
            committed: 0,
            used: 0,
            first: date,
            last: date,
            known: false,
            source: String::new(),
            example: String::new(),
        }
    }
}
