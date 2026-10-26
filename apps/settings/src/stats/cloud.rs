//! 云服务用量：累计数（`api-usage.json`）与按天、今天按小时的请求和 Token。

use std::collections::BTreeMap;

use jiff::civil::Date;
use qingjian_learning::figures;
use qingjian_predict::{ApiUsageHour, ApiUsageStats};

#[derive(Debug, Clone, Default)]
pub struct CloudStats {
    pub today: Date,

    /// 现在是几点（0–23）。
    pub hour: u8,

    /// 从有记录起的累计；文件读不出来为 `None`。
    pub totals: Option<ApiUsageStats>,

    /// 每天的用量；按小时记是这一版才有的，更早的只在累计里。
    pub days: BTreeMap<Date, ApiUsageHour>,

    /// 过去 24 小时各个小时的用量，最后一格是现在这个钟点（见 [`figures::last_24`]）。
    pub hours: [ApiUsageHour; 24],
}

impl CloudStats {
    pub fn today_usage(&self) -> ApiUsageHour {
        self.days.get(&self.today).copied().unwrap_or_default()
    }

    /// 近 7 天（含今天），与输入层同一个口径。
    pub fn recent(&self) -> ApiUsageHour {
        figures::recent(&self.days, self.today)
    }

    pub fn month(&self) -> ApiUsageHour {
        figures::month(&self.days, self.today)
    }

    /// Token 最多的一天。
    pub fn best_day(&self) -> Option<(Date, ApiUsageHour)> {
        self.days
            .iter()
            .filter(|(_, day)| day.total_tokens > 0)
            .max_by_key(|(_, day)| day.total_tokens)
            .map(|(date, day)| (*date, *day))
    }
}
