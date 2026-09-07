use super::LevelCount;

/// 一种学习语言的词汇汇总。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VocabularySummary {
    /// 见过的不同译词数。
    pub seen: u64,

    /// 其中看熟了的（见过的轮次不少于 [`super::FRESH_UNTIL`]）。
    pub familiar: u64,

    /// 上屏过带这条译词的候选的不同译词数。
    pub committed: u64,

    /// 直接打出过的不同译词数。
    pub used: u64,

    /// 最近 7 天（含今天）第一次见到的译词数。
    pub new_this_week: u64,

    /// 按等级（从易到难）分的数字；没有等级表时为空。
    pub levels: Vec<LevelCount>,
}
