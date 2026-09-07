use super::Usage;

/// 输入统计的汇总，偏好设置「统计」页显示的就是它。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UsageSummary {
    /// 今天。
    pub today: Usage,

    /// 最近 7 天（含今天）。
    pub week: Usage,

    /// 从有记录起的累计。
    pub total: Usage,

    /// 有过上屏的天数。
    pub days: u32,

    /// 最早一条记录的日期（`YYYY-MM-DD`）；没记录为 `None`。
    pub since: Option<String>,
}
