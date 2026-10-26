//! 由按天、按小时的用量算出来给人看的数：近 7 天、速度、几个比例、最多与最快的一天。
//! PC 设置程序的统计页与安卓 App 的首页、统计页都用这一套，两端口径一致（以后多端同步也照这个算）。

use std::collections::BTreeMap;
use std::ops::AddAssign;

use jiff::civil::Date;
use qingjian_core::Usage;

/// 「近 7 天」算几天（含今天）。滚动的，不按周一或周日归零；生词本的「近 7 天新见」同一个口径。
pub const RECENT_DAYS: i64 = 7;

/// 算「今天速度」时至少打这么久（毫秒），免得几个字就出个离谱的数。
pub const TODAY_MIN_MS: u64 = 30_000;

/// 算平均速度与「最快的一天」时至少打这么久（毫秒）。
pub const AVERAGE_MIN_MS: u64 = 5 * 60_000;

/// 「近 7 天」的头一天。
pub fn recent_start(today: Date) -> Date {
    today.saturating_sub(jiff::Span::new().days(RECENT_DAYS - 1))
}

/// `from` 到 `to`（含两头）这几天加起来。
pub fn sum_days<T: Copy + Default + AddAssign>(
    days: &BTreeMap<Date, T>,
    from: Date,
    to: Date,
) -> T {
    let mut total = T::default();
    for value in days.range(from..=to).map(|(_, value)| *value) {
        total += value;
    }
    total
}

/// 近 7 天（含今天）加起来。
pub fn recent<T: Copy + Default + AddAssign>(days: &BTreeMap<Date, T>, today: Date) -> T {
    sum_days(days, recent_start(today), today)
}

/// 这个月 1 号到今天加起来。
pub fn month<T: Copy + Default + AddAssign>(days: &BTreeMap<Date, T>, today: Date) -> T {
    sum_days(days, today.first_of_month(), today)
}

/// 全部记录加起来。
pub fn total(days: &BTreeMap<Date, Usage>) -> Usage {
    days.values().fold(Usage::default(), |sum, day| sum + *day)
}

/// 过去 24 小时：从昨天这个钟点的下一个小时排到现在这个钟点（最后一格）。
pub fn last_24<T: Copy>(yesterday: [T; 24], today: [T; 24], hour: u8) -> [T; 24] {
    std::array::from_fn(|index| {
        let back = 23 - index;
        let hour = usize::from(hour);
        if back <= hour {
            today[hour - back]
        } else {
            yesterday[hour + 24 - back]
        }
    })
}

/// 有输入的天数。
pub fn active_days(days: &BTreeMap<Date, Usage>) -> usize {
    days.values().filter(|day| day.hanzi > 0).count()
}

/// 有输入的日子平均每天几个汉字；一天都没有为 `None`。
pub fn daily_average(days: &BTreeMap<Date, Usage>) -> Option<u64> {
    total(days).hanzi.checked_div(active_days(days) as u64)
}

/// 汉字最多的一天。
pub fn best_day(days: &BTreeMap<Date, Usage>) -> Option<(Date, Usage)> {
    days.iter()
        .filter(|(_, day)| day.hanzi > 0)
        .max_by_key(|(_, day)| day.hanzi)
        .map(|(date, day)| (*date, *day))
}

/// 打得最快的一天（那天至少打了 [`AVERAGE_MIN_MS`]）。
pub fn fastest_day(days: &BTreeMap<Date, Usage>) -> Option<(Date, f64)> {
    days.iter()
        .filter_map(|(date, day)| speed(day, AVERAGE_MIN_MS).map(|speed| (*date, speed)))
        .max_by(|a, b| a.1.total_cmp(&b.1))
}

/// 速度（字 / 分钟）：记了时长的那些上屏打出的汉字除以打字时长，老记录与原样上屏只有字数、不在里面。
/// 打字不到 `min_ms` 不算。
pub fn speed(usage: &Usage, min_ms: u64) -> Option<f64> {
    (usage.typing_ms >= min_ms)
        .then(|| usage.typed_hanzi as f64 * 60_000.0 / usage.typing_ms as f64)
}

/// 选的是首选的比例。
pub fn first_choice_rate(usage: &Usage) -> Option<f64> {
    (usage.choices > 0).then(|| usage.first_choices as f64 / usage.choices as f64)
}

/// 上屏前没退格、没删过的比例（一次打对）。
pub fn clean_rate(usage: &Usage) -> Option<f64> {
    (usage.choices > 0).then(|| 1.0 - usage.edited as f64 / usage.choices as f64)
}

/// 每个字平均敲几键（与速度同一批上屏）。
pub fn keys_per_hanzi(usage: &Usage) -> Option<f64> {
    (usage.keys > 0 && usage.typed_hanzi > 0).then(|| usage.keys as f64 / usage.typed_hanzi as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn date(text: &str) -> Date {
        text.parse().unwrap()
    }

    #[test]
    fn recent_is_seven_rolling_days_and_month_starts_on_the_first() {
        // 2026-10-15 是周四：近 7 天是 10-09 到 10-15，不管周几
        let today = date("2026-10-15");
        let days: BTreeMap<Date, Usage> = [
            ("2026-09-30", 1),
            ("2026-10-01", 10),
            ("2026-10-08", 100),
            ("2026-10-09", 1_000),
            ("2026-10-15", 10_000),
        ]
        .into_iter()
        .map(|(text, hanzi)| (date(text), day(hanzi, 600_000)))
        .collect();
        assert_eq!(recent_start(today), date("2026-10-09"));
        assert_eq!(recent(&days, today).hanzi, 11_000);
        assert_eq!(month(&days, today).hanzi, 11_110);
        assert_eq!(total(&days).hanzi, 11_111);
        assert_eq!(active_days(&days), 5);
        assert_eq!(daily_average(&days), Some(2_222));
        assert_eq!(best_day(&days).unwrap().0, today);
        assert_eq!(fastest_day(&days).unwrap().0, today);
        assert_eq!(daily_average(&BTreeMap::new()), None);
    }

    #[test]
    fn last_24_hours_end_at_the_current_one() {
        let yesterday: [u32; 24] = std::array::from_fn(|hour| 100 + hour as u32);
        let today: [u32; 24] = std::array::from_fn(|hour| hour as u32);
        let hours = last_24(yesterday, today, 2);
        // 昨天 3 点到 23 点，接着今天 0 点到 2 点
        assert_eq!(hours[0], 103);
        assert_eq!(hours[20], 123);
        assert_eq!(hours[21], 0);
        assert_eq!(hours[23], 2);
        assert_eq!(last_24(yesterday, today, 23), today);
    }

    #[test]
    fn speed_needs_enough_typing_time() {
        let usage = day(10, 60_000);
        assert_eq!(speed(&usage, TODAY_MIN_MS), Some(10.0));
        assert_eq!(speed(&usage, AVERAGE_MIN_MS), None);
        // 老版本记的字数没有时长：不进速度
        let mut legacy = usage;
        legacy.hanzi += 90_000;
        assert_eq!(speed(&legacy, TODAY_MIN_MS), Some(10.0));
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
        assert_eq!(clean_rate(&old), None);
        assert_eq!(keys_per_hanzi(&old), None);
    }
}
