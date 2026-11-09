//! 输入量：按天的用量与今天按小时的用量；近 7 天、本月、最多的一天、速度这些派生数都交给
//! [`qingjian_learning::figures`]，与安卓 App 同一套口径。

use std::collections::BTreeMap;

use jiff::civil::Date;
use qingjian_core::Usage;
use qingjian_learning::figures;

#[derive(Debug, Clone, Default)]
pub struct InputStats {
    pub today: Date,

    /// 现在是几点（0–23）。
    pub hour: u8,

    /// 每天的用量，按日期排好。
    pub days: BTreeMap<Date, Usage>,

    /// 过去 24 小时各个小时的用量，最后一格是现在这个钟点（见 [`figures::last_24`]）。
    pub hours: [Usage; 24],
}

impl InputStats {
    pub fn on(&self, date: Date) -> Usage {
        self.days.get(&date).copied().unwrap_or_default()
    }

    pub fn today_usage(&self) -> Usage {
        self.on(self.today)
    }

    /// 近 7 天（含今天）。
    pub fn recent(&self) -> Usage {
        figures::recent(&self.days, self.today)
    }

    pub fn month(&self) -> Usage {
        figures::month(&self.days, self.today)
    }

    pub fn total(&self) -> Usage {
        figures::total(&self.days)
    }

    /// 有输入的天数与最早那天。
    pub fn active_days(&self) -> (usize, Option<Date>) {
        (
            figures::active_days(&self.days),
            self.days.keys().next().copied(),
        )
    }

    /// 今天的速度（字 / 分钟）。
    pub fn today_speed(&self) -> Option<f64> {
        figures::speed(&self.today_usage(), figures::TODAY_MIN_MS)
    }

    /// 全部记录的平均速度。
    pub fn average_speed(&self) -> Option<f64> {
        figures::speed(&self.total(), figures::AVERAGE_MIN_MS)
    }
}
