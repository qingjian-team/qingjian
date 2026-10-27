//! 日期、时间、星期的几种常用写法。

use jiff::Zoned;
use jiff::civil::{Date, Weekday};
use jiff::tz::TimeZone;

use super::numeral::chinese_lower;

/// 表达式模式里敲的日期（`v1998-3-8`）：四位年份 + 月 + 日，分隔符 `-` / `/` / `.` 任意，
/// 返回中文写法在前（一九九八年三月八日 / 1998年3月8日 / 1998-03-08）。
///
/// **年份必须四位**，`v1-2-3` 这种才仍按算式算、不被当成 1 年 2 月 3 日；月日超出该月天数
/// （`1998-2-30`）也不认，同样回落到算式。多于三段（`1998-3-8-1`）不是日期。
pub fn input_date_forms(body: &str) -> Option<Vec<String>> {
    let mut parts = body.split(['-', '/', '.']);
    let (year, month, day) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() {
        return None;
    }
    let year: i16 = year.parse().ok()?;
    let month: i8 = month.parse().ok()?;
    let day: i8 = day.parse().ok()?;
    if !(1000..=9999).contains(&year) {
        return None;
    }
    // 用 jiff 判月日是否真有这一天（2 月 30 日、13 月这类都不认）
    let date = Date::new(year, month, day).ok()?;
    Some(vec![
        format!(
            "{}年{}月{}日",
            chinese_year(year),
            chinese_lower(&date.month().to_string()),
            chinese_lower(&date.day().to_string())
        ),
        format!("{year}年{month}月{day}日"),
        format!("{year}-{month:02}-{day:02}"),
    ])
}

/// 年份逐位读：`1998` → 一九九八，`2006` → 二〇〇六（年份里的零按日期惯例写「〇」）。
fn chinese_year(year: i16) -> String {
    const DIGITS: [&str; 10] = ["〇", "一", "二", "三", "四", "五", "六", "七", "八", "九"];
    year.to_string()
        .bytes()
        .map(|b| DIGITS[usize::from(b - b'0')])
        .collect()
}

/// `rq`：`2026年9月3日`、`2026-09-03`、`2026/09/03`。
pub fn date_forms(now: &Zoned) -> Vec<String> {
    let (year, month, day) = (now.year(), now.month(), now.day());
    vec![
        format!("{year}年{month}月{day}日"),
        format!("{year}-{month:02}-{day:02}"),
        format!("{year}/{month:02}/{day:02}"),
    ]
}

/// `sj`：`19:06`、`19:06:23`、`19点06分`、`20260903190623`（时间戳 `yyyyMMddHHmmss`）。
pub fn time_forms(now: &Zoned) -> Vec<String> {
    let (year, month, day) = (now.year(), now.month(), now.day());
    let (hour, minute, second) = (now.hour(), now.minute(), now.second());
    vec![
        format!("{hour:02}:{minute:02}"),
        format!("{hour:02}:{minute:02}:{second:02}"),
        format!("{hour}点{minute:02}分"),
        format!("{year:04}{month:02}{day:02}{hour:02}{minute:02}{second:02}"),
    ]
}

/// 表达式键之后是这个词时出时间戳（`vts`）。
pub const TIMESTAMP_KEYWORD: &str = "ts";

/// `vts`：Unix 秒、UTC（RFC 3339，`2026-09-03T11:06:23Z`）、Unix 毫秒、带时区的 ISO 8601（`2026-09-03T19:06:23+08:00`）。
pub fn timestamp_forms(now: &Zoned) -> Vec<String> {
    let instant = now.timestamp();
    vec![
        instant.as_second().to_string(),
        now.with_time_zone(TimeZone::UTC)
            .strftime("%Y-%m-%dT%H:%M:%SZ")
            .to_string(),
        instant.as_millisecond().to_string(),
        now.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string(),
    ]
}

/// `xq`：`星期四`、`周四`。
pub fn weekday_forms(now: &Zoned) -> Vec<String> {
    let name = match now.weekday() {
        Weekday::Monday => "一",
        Weekday::Tuesday => "二",
        Weekday::Wednesday => "三",
        Weekday::Thursday => "四",
        Weekday::Friday => "五",
        Weekday::Saturday => "六",
        Weekday::Sunday => "日",
    };
    vec![format!("星期{name}"), format!("周{name}")]
}
