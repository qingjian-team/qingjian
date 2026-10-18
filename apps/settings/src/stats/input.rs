//! 输入量：按天的用量与今天按小时的用量，及由它们算出的本周、本月、最高一天、速度与几个比例。

use std::collections::BTreeMap;

use jiff::civil::Date;
use qingjian_core::Usage;

use super::{first_of_month, speed, sum_days, week_start};

/// 算「最快一天」时，一天至少打这么久才算（毫秒）。
const FASTEST_MIN_MS: u64 = 5 * 60_000;

/// 算「今天速度」时至少打这么久（毫秒）。
const TODAY_MIN_MS: u64 = 30_000;

#[derive(Debug, Clone, Default)]
pub struct InputStats {
    pub today: Date,

    /// 现在是几点（0–23）。
    pub hour: u8,

    /// 每天的用量，按日期排好。
    pub days: BTreeMap<Date, Usage>,

    /// 过去 24 小时各个小时的用量，最后一格是现在这个钟点（见 [`super::last_24`]）。
    pub hours: [Usage; 24],
}

impl InputStats {
    pub fn on(&self, date: Date) -> Usage {
        self.days.get(&date).copied().unwrap_or_default()
    }

    pub fn today_usage(&self) -> Usage {
        self.on(self.today)
    }

    pub fn week(&self) -> Usage {
        sum_days(&self.days, week_start(self.today), self.today)
    }

    pub fn month(&self) -> Usage {
        sum_days(&self.days, first_of_month(self.today), self.today)
    }

    pub fn total(&self) -> Usage {
        self.days
            .values()
            .fold(Usage::default(), |sum, day| sum + *day)
    }

    /// 有输入的天数与最早那天。
    pub fn active_days(&self) -> (usize, Option<Date>) {
        let days = self.days.values().filter(|day| day.hanzi > 0).count();
        (days, self.days.keys().next().copied())
    }

    /// 汉字最多的一天。
    pub fn best_day(&self) -> Option<(Date, Usage)> {
        self.days
            .iter()
            .filter(|(_, day)| day.hanzi > 0)
            .max_by_key(|(_, day)| day.hanzi)
            .map(|(date, day)| (*date, *day))
    }

    /// 今天的速度（字 / 分钟）。
    pub fn today_speed(&self) -> Option<f64> {
        speed(&self.today_usage(), TODAY_MIN_MS)
    }

    /// 全部记录的平均速度。
    pub fn average_speed(&self) -> Option<f64> {
        speed(&self.total(), FASTEST_MIN_MS)
    }

    /// 打得最快的一天。
    pub fn fastest_day(&self) -> Option<(Date, f64)> {
        self.days
            .iter()
            .filter_map(|(date, day)| speed(day, FASTEST_MIN_MS).map(|speed| (*date, speed)))
            .max_by(|a, b| a.1.total_cmp(&b.1))
    }
}

/// 选的是首选的比例。
pub fn first_choice_rate(usage: &Usage) -> Option<f64> {
    (usage.choices > 0).then(|| usage.first_choices as f64 / usage.choices as f64)
}

/// 上屏前没退格、没删过的比例。
pub fn clean_rate(usage: &Usage) -> Option<f64> {
    (usage.choices > 0).then(|| 1.0 - usage.edited as f64 / usage.choices as f64)
}

/// 每个字平均敲几键（与速度同一批上屏）。
pub fn keys_per_hanzi(usage: &Usage) -> Option<f64> {
    (usage.keys > 0 && usage.typed_hanzi > 0).then(|| usage.keys as f64 / usage.typed_hanzi as f64)
}

#[cfg(test)]
mod tests {
    use super::{InputStats, clean_rate, first_choice_rate, keys_per_hanzi};
    use qingjian_core::Usage;

    fn day(hanzi: u64, typing_ms: u64) -> Usage {
        Usage {
            hanzi,
            commits: 1,
            choices: 4,
            first_choices: 3,
            typed_hanzi: hanzi,
            keys: hanzi * 3,
            typing_ms,
            edited: 1,
            ..Usage::default()
        }
    }

    #[test]
    fn week_starts_on_sunday_and_month_on_the_first() {
        // 2026-10-15 是周四
        let today = "2026-10-15".parse().unwrap();
        let mut stats = InputStats {
            today,
            ..InputStats::default()
        };
        for (date, hanzi) in [
            ("2026-09-30", 1),
            ("2026-10-01", 10),
            ("2026-10-11", 100),
            ("2026-10-12", 1_000),
            ("2026-10-15", 10_000),
        ] {
            stats
                .days
                .insert(date.parse().unwrap(), day(hanzi, 600_000));
        }
        assert_eq!(stats.week().hanzi, 11_100);
        assert_eq!(stats.month().hanzi, 11_110);
        assert_eq!(stats.total().hanzi, 11_111);
        assert_eq!(stats.best_day().unwrap().0, today);
        assert_eq!(stats.today_speed(), Some(1_000.0));
        // 今天另有老版本记的 9 万字（没有时长）：不进速度
        let mut legacy = stats.on(today);
        legacy.hanzi += 90_000;
        stats.days.insert(today, legacy);
        assert_eq!(stats.today_speed(), Some(1_000.0));
        assert_eq!(stats.active_days().0, 5);
    }

    #[test]
    fn rates_need_new_records() {
        let usage = day(10, 1_000);
        assert_eq!(first_choice_rate(&usage), Some(0.75));
        assert_eq!(clean_rate(&usage), Some(0.75));
        assert_eq!(keys_per_hanzi(&usage), Some(3.0));
        // 老的按天记录只有字数
        let old = Usage {
            hanzi: 10,
            commits: 3,
            ..Usage::default()
        };
        assert_eq!(first_choice_rate(&old), None);
        assert_eq!(keys_per_hanzi(&old), None);
    }
}
