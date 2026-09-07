use std::time::Duration;

/// 一次候选查询各阶段的耗时。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Timings {
    /// 拼音切分。
    pub parse: Duration,

    /// 词库查询（含所有切分）。
    pub lookup: Duration,

    /// 排序与去重。
    pub rank: Duration,
}

impl Timings {
    pub fn total(&self) -> Duration {
        self.parse + self.lookup + self.rank
    }
}
