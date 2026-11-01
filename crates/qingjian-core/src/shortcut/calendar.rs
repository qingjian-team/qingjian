//! 日期、时间、星期的各种常用写法。
//!
//! `rq` / `sj` / `xq` 是输入法里最高频的三个快捷词，所以这里给得比"够用"多一档：
//! 中文与数字两套、带不带星期、短格式与完整格式、12 / 24 小时制、纯数字串，
//! 各平台导数据（Excel 认 `2026/11/1`、数据库认 `2026-11-01`、文件名爱用 `20261101`）都能直接抄。
//! 顺序按"最可能是想要的那个"排，第一条永远是当前最常用的那个写法。

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

/// `rq`：今天。中文与数字两套、带星期、月日、纯数字串、两位年、欧式与美式、ISO 周。
pub fn date_forms(now: &Zoned) -> Vec<String> {
    let (year, month, day) = (now.year(), now.month(), now.day());
    let weekday = weekday_name(now.weekday());
    let weekday_long = weekday_long_name(now.weekday());
    // ISO 8601 的周号：`2026-W44-7`
    let iso = now.date().iso_week_date();
    vec![
        format!("{year}年{month}月{day}日"),
        format!("{year}-{month:02}-{day:02}"),
        format!("{year}/{month:02}/{day:02}"),
        format!("{year}年{month}月{day}日 {weekday_long}"),
        format!("{year}-{month:02}-{day:02} {weekday}"),
        format!("{year}.{month:02}.{day:02}"),
        format!("{month}月{day}日"),
        format!("{year:04}{month:02}{day:02}"),
        // 两位年：文件名、表头里常见
        format!("{:02}-{month:02}-{day:02}", year % 100),
        // 欧式 日/月/年 与美式 月/日/年，两份都给，看清楚再选
        format!("{day:02}/{month:02}/{year}"),
        format!("{month:02}/{day:02}/{year}"),
        format!(
            "{}-W{:02}-{}",
            iso.year(),
            iso.week(),
            iso.weekday().to_monday_one_offset()
        ),
        // 中文年月日逐位
        format!(
            "{}年{}月{}日",
            chinese_year(year),
            chinese_lower(&month.to_string()),
            chinese_lower(&day.to_string())
        ),
    ]
}

/// 时间快捷词的输入码（`sj`）。壳按 [`super::is_live_time_keyword`] 判断要不要随钟重查。
pub const TIME_KEYWORD: &str = "sj";

/// `sj`：现在。24 / 12 小时制、带秒、带毫秒、中文读法、纯数字、完整日期时间、文件名友好格式。
pub fn time_forms(now: &Zoned) -> Vec<String> {
    let (year, month, day) = (now.year(), now.month(), now.day());
    let (hour, minute, second) = (now.hour(), now.minute(), now.second());
    let millis = now.millisecond();
    let (hour12, meridiem_cn, meridiem_en) = match hour {
        0 => (12, "午夜", "AM"),
        1..=11 => (hour, "上午", "AM"),
        12 => (12, "中午", "PM"),
        _ => (hour - 12, "下午", "PM"),
    };
    vec![
        // 前三条是原有顺序，不动它：常用写法始终排在最前
        format!("{hour:02}:{minute:02}"),
        format!("{hour:02}:{minute:02}:{second:02}"),
        format!("{hour}点{minute:02}分"),
        // 以下是补的写法，毫秒放补的第一条：打点记时间的人不用翻页
        format!("{hour:02}:{minute:02}:{second:02}.{millis:03}"),
        format!("{year}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}.{millis:03}"),
        format!("{year}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}"),
        format!("{hour}点{minute:02}分{second:02}秒"),
        // 12 小时制：中文口语与英文惯例各一份
        format!("{meridiem_cn}{hour12}:{minute:02}"),
        format!("{hour12}:{minute:02} {meridiem_en}"),
        // 纯数字：`170725123` 带毫秒、`170725`、`1707`、14 位时间戳
        format!("{hour:02}{minute:02}{second:02}{millis:03}"),
        format!("{hour:02}{minute:02}{second:02}"),
        format!("{hour:02}{minute:02}"),
        format!("{year:04}{month:02}{day:02}{hour:02}{minute:02}{second:02}"),
        // 文件名 / 日志时间戳友好
        format!("{year:04}{month:02}{day:02}_{hour:02}{minute:02}{second:02}"),
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

/// `xq`：`星期四`、`周四`、`礼拜四`。
pub fn weekday_forms(now: &Zoned) -> Vec<String> {
    let name = weekday_name(now.weekday());
    vec![
        format!("星期{name}"),
        format!("周{name}"),
        format!("礼拜{name}"),
    ]
}

/// `星期一` 里的那个字（周日取「日」而不是「天」，与日历、报刊一致）。
fn weekday_name(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Monday => "一",
        Weekday::Tuesday => "二",
        Weekday::Wednesday => "三",
        Weekday::Thursday => "四",
        Weekday::Friday => "五",
        Weekday::Saturday => "六",
        Weekday::Sunday => "日",
    }
}

/// 完整写法：`星期日`。
fn weekday_long_name(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Monday => "星期一",
        Weekday::Tuesday => "星期二",
        Weekday::Wednesday => "星期三",
        Weekday::Thursday => "星期四",
        Weekday::Friday => "星期五",
        Weekday::Saturday => "星期六",
        Weekday::Sunday => "星期日",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed() -> Zoned {
        "2026-11-01T17:07:25.123+08:00[Asia/Shanghai]"
            .parse()
            .unwrap()
    }

    #[test]
    fn date_covers_common_layouts() {
        let forms = date_forms(&fixed());
        assert_eq!(forms[0], "2026年11月1日");
        assert_eq!(forms[1], "2026-11-01");
        assert_eq!(forms[2], "2026/11/01");
        assert!(forms.iter().any(|f| f == "2026年11月1日 星期日"));
        assert!(forms.iter().any(|f| f == "20261101"));
        assert!(forms.iter().any(|f| f == "26-11-01"));
        assert!(forms.iter().any(|f| f == "01/11/2026"));
        assert!(forms.iter().any(|f| f == "二〇二六年十一月一日"));
    }

    #[test]
    fn time_covers_common_layouts() {
        let forms = time_forms(&fixed());
        assert_eq!(forms[0], "17:07");
        assert_eq!(forms[1], "17:07:25");
        assert_eq!(forms[2], "17点07分");
        assert_eq!(forms[3], "17:07:25.123");
        assert!(forms.iter().any(|f| f == "2026-11-01 17:07:25"));
        assert!(forms.iter().any(|f| f == "下午5:07"));
        assert!(forms.iter().any(|f| f == "5:07 PM"));
        assert!(forms.iter().any(|f| f == "170725"));
        assert!(forms.iter().any(|f| f == "170725123"));
    }

    #[test]
    fn weekday_has_three_spellings() {
        let forms = weekday_forms(&fixed());
        assert_eq!(forms, vec!["星期日", "周日", "礼拜日"]);
    }
}
