/// 词汇汇总里一个等级的数字。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LevelCount {
    /// 等级名（`A1` / `N5`）。
    pub name: String,

    /// 等级表里这一级共有多少词。
    pub total: u64,

    /// 其中见过的。
    pub seen: u64,

    /// 其中看熟了的。
    pub familiar: u64,

    /// 其中上屏过的。
    pub committed: u64,
}
