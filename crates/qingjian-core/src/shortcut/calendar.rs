//! 日期、时间、星期的几种常用写法。

use jiff::Zoned;
use jiff::civil::Weekday;

/// `rq`：`2026年9月3日`、`2026-09-03`、`2026/09/03`。
pub fn date_forms(now: &Zoned) -> Vec<String> {
    let (year, month, day) = (now.year(), now.month(), now.day());
    vec![
        format!("{year}年{month}月{day}日"),
        format!("{year}-{month:02}-{day:02}"),
        format!("{year}/{month:02}/{day:02}"),
    ]
}

/// `sj`：`19:06`、`19:06:23`、`19点06分`。
pub fn time_forms(now: &Zoned) -> Vec<String> {
    let (hour, minute, second) = (now.hour(), now.minute(), now.second());
    vec![
        format!("{hour:02}:{minute:02}"),
        format!("{hour:02}:{minute:02}:{second:02}"),
        format!("{hour}点{minute:02}分"),
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
