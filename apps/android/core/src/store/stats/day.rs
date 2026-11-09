//! 统计详情页柱状图里的一天。

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct StatsDay {
    /// `2026-10-19`。
    pub date: String,

    pub hanzi: u64,
}
