//! 统计详情页（参照 PC 设置程序的统计页，图表简化成两张柱状图）。

use serde::Serialize;

use super::{StatsCount, StatsDay, StatsVocabulary};

#[derive(Debug, Serialize)]
pub struct StatsDetail {
    pub today: StatsCount,

    /// 近 7 天（含今天），口径见 `qingjian_learning::figures::recent`。
    pub recent: StatsCount,

    /// 这个月 1 号起。
    pub month: StatsCount,

    pub total: StatsCount,

    /// 过去 24 小时每小时的汉字数，最后一格是现在这个钟点。
    pub hours: Vec<u64>,

    /// 现在是几点（0–23），柱状图标钟点用。
    pub hour: u8,

    /// 最近 30 天（含今天）每天的汉字数，早的在前。
    pub days: Vec<StatsDay>,

    /// 汉字最多的一天；还没有输入为 `None`。
    pub best_day: Option<StatsDay>,

    /// 有输入汉字的天数。
    pub active_days: u64,

    /// 有输入的日子平均每天几个汉字；还没有输入为 0。
    pub average: u64,

    /// 速度（字 / 分钟）：今天与全部记录；打字时间太短为 `None`。
    pub today_speed: Option<f64>,

    pub average_speed: Option<f64>,

    /// 选词时选的是第一个的比例（0–1）。
    pub first_choice_rate: Option<f64>,

    /// 选词上屏前没退格、没删过的比例（0–1）。
    pub clean_rate: Option<f64>,

    /// 每个汉字平均敲几键。
    pub keys_per_hanzi: Option<f64>,

    /// 学习语言关着为 `None`。
    pub vocabulary: Option<StatsVocabulary>,
}
