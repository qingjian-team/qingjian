//! 统计详情页词汇一节里的一级。

use qingjian_core::LevelCount;
use serde::Serialize;

/// 一级（`A1`、`N5`）：等级表里共有多少词，其中见过、看熟、上屏过的。
#[derive(Debug, Serialize)]
pub struct StatsLevel {
    pub name: String,

    pub total: u64,

    pub seen: u64,

    pub familiar: u64,

    pub committed: u64,
}

impl From<LevelCount> for StatsLevel {
    fn from(level: LevelCount) -> Self {
        Self {
            name: level.name,
            total: level.total,
            seen: level.seen,
            familiar: level.familiar,
            committed: level.committed,
        }
    }
}
