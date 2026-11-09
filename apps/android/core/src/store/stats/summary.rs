//! 「我的」页的一组数字。

use serde::Serialize;

use super::StatsCount;

/// 今天、近 7 天、累计的汉字数与英文词数，以及有输入的天数。
#[derive(Debug, Default, Serialize)]
pub struct StatsSummary {
    pub today: StatsCount,

    pub recent: StatsCount,

    pub total: StatsCount,

    pub days: u32,
}
