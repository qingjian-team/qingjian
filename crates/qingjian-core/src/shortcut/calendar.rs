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
pub fn input_date_forms(body: &str, now: &Zoned) -> Option<Vec<String>> {
    // 先试 `now` 及其偏移（`vnow`、`vnow+4`、`vnow;-4`），再试「日期 ± 天数」，最后普通日期
    now_forms(body, now)
        .or_else(|| shift_by_days(body))
        .or_else(|| date_forms_of(body))
}

/// `vnow`：今天；`vnow+4` / `vnow-4` / `vnow;-4` 是相对今天加减天数。
///
/// 输出走与 `v2026.11.5` 同一套日期格式化，所以候选条与打一个具体日期完全一样。
fn now_forms(body: &str, now: &Zoned) -> Option<Vec<String>> {
    let days = parse_now_offset(body.strip_prefix("now")?)?;
    let moved = now.date().checked_add(jiff::Span::new().days(days)).ok()?;
    date_forms_of(&format!(
        "{}.{}.{}",
        moved.year(),
        moved.month(),
        moved.day()
    ))
}

/// `now` 后面的偏移：空（今天）、`+4`、`-4`，前面可以隔一个分号或冒号（`;4`、`;-4`、`：-4`）。
///
/// 认分号是因为键盘上冒号就是 Shift + 分号，有些布局送到缓冲区的是分号；
/// 不写符号时按往后算（`vnow;4` 是四天后）。
fn parse_now_offset(rest: &str) -> Option<i64> {
    if rest.is_empty() {
        return Some(0);
    }
    let trimmed = rest.trim_start_matches([';', '；', ':', '：']);
    if trimmed.is_empty() {
        return None;
    }
    let (sign, digits) = match trimmed.as_bytes()[0] {
        b'+' => (1, &trimmed[1..]),
        b'-' => (-1, &trimmed[1..]),
        _ => (1, trimmed),
    };
    if digits.is_empty() || digits.len() > 4 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let count: i64 = digits.parse().ok()?;
    Some(sign * count)
}

/// 末尾带天数偏移的日期：`1234.9.9-7`（往前 7 天）、`1234.9.9+30`（往后 30 天）。
///
/// 只认完整年月日（三段）—— 加减到天才有意义。年份要求正数：公元前那个负号
/// 既像前缀又像减号，而且与我们用的天文纪年差一年，不在这里算。
fn shift_by_days(body: &str) -> Option<Vec<String>> {
    let (head, days) = split_day_offset(body)?;
    // 偏移的减号不能和日期的分隔符撞车：`1998-3-8-1` 到底是「四段日期」还是
    // 「1998-3-8 减一天」，分不开。按既有约定（多于三段不是日期）交给算式。
    // `1234.9.9-7`、`1234/9/9+7` 这种分隔符不同，没有歧义。
    if days < 0 && head.contains('-') {
        return None;
    }
    let mut parts = head.split(['.', '-', '/']);
    let year: i16 = parts.next()?.parse().ok()?;
    let month: i8 = parts.next()?.parse().ok()?;
    let day: i8 = parts.next()?.parse().ok()?;
    // 多于三段不是日期（`1234.9.9-7-8`）
    if parts.next().is_some() || year < 100 {
        return None;
    }
    let date = jiff::civil::Date::new(year, month, day).ok()?;
    let moved = date.checked_add(jiff::Span::new().days(days)).ok()?;
    // 归一化成点分隔再走普通日期解析，输出与 `v1998.3.8` 完全同一套写法
    date_forms_of(&format!(
        "{}.{}.{}",
        moved.year(),
        moved.month(),
        moved.day()
    ))
}

/// 末尾的 `+N` / `-N`（天数）与它前面的日期部分。
fn split_day_offset(body: &str) -> Option<(&str, i64)> {
    let at = body.rfind(['+', '-'])?;
    if at == 0 || at + 1 >= body.len() {
        return None;
    }
    let (head, tail) = body.split_at(at);
    let digits = &tail[1..];
    if digits.is_empty() || digits.len() > 4 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let count: i64 = digits.parse().ok()?;
    let signed = if tail.starts_with('-') { -count } else { count };
    Some((head, signed))
}

fn date_forms_of(body: &str) -> Option<Vec<String>> {
    // 负号开头是公元前：`-221.6.1`、`-210`
    let (negative, rest) = match body.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, body),
    };
    // 分隔符三种都认：`.`、`-`、`/`。同一段里只许用一种（`v1998-3.8` 混用，交给算式），
    // 否则 `1998-3.8` 该切成什么就说不清了。一个分隔符都没有时是单年份（`v1632`）。
    //
    // `-` 与 `/` 既可以是日期分隔符也可以是减号：`v1998-3-8` 同时给日期写法与算式结果，
    // 日期在前、算式在后，由用户挑。认不出来的写法（`v1-2-3`）门槛挡住。
    let used: Vec<char> = ['.', '-', '/']
        .into_iter()
        .filter(|sep| rest.contains(*sep))
        .collect();
    if used.len() > 1 {
        return None;
    }
    let parts: Vec<&str> = match used.first() {
        Some(sep) => rest.split(*sep).collect(),
        None => vec![rest],
    };
    if parts.len() > 3 {
        return None;
    }
    let year: i16 = parts[0].parse().ok()?;
    let year = if negative { -year } else { year };
    if year == 0 || year.unsigned_abs() > 9999 {
        return None;
    }
    // 门槛按段数分：单年份与年月都要四位（`v1632` 查崇祯五年，`v123` 仍按算式），
    // 三段完整日期放到三位（`v755.4.1` 是天宝十四年）
    if !negative {
        // 单年份与年月都要四位（`v1632` 能查崇祯五年，`v123` 仍是纯数字）；
        // 三段完整日期放到三位（`v755.4.1` 是天宝十四年）
        let floor = match parts.len() {
            3 => 100,
            _ => 1000,
        };
        if year < floor {
            return None;
        }
    }
    // 月日按输入给几段算几段，缺的补 1 只为校验合法性，显示时按原段数
    let month: i8 = match parts.get(1) {
        Some(text) => text.parse().ok()?,
        None => 1,
    };
    let day: i8 = match parts.get(2) {
        Some(text) => text.parse().ok()?,
        None => 1,
    };
    let date = Date::new(year, month, day).ok()?;
    let has_month = parts.len() >= 2;
    let has_day = parts.len() >= 3;
    // 中文明文：前二一〇年 / 二月 / 一日，按实际段数拼
    let middle = if has_day {
        format!(
            "{}年{}月{}日",
            chinese_year(year),
            chinese_lower(&date.month().to_string()),
            chinese_lower(&date.day().to_string())
        )
    } else if has_month {
        format!(
            "{}年{}月",
            chinese_year(year),
            chinese_lower(&date.month().to_string())
        )
    } else {
        format!("{}年", chinese_year(year))
    };
    // 三种分隔符都给：`-` 给数据库，`/` 给 Excel，`.` 跟输入写法一致
    let numeric = if has_day {
        format!("{year}-{month:02}-{day:02}")
    } else if has_month {
        format!("{year}-{month:02}")
    } else {
        year.to_string()
    };
    let slashed = if has_day {
        format!("{year}/{month:02}/{day:02}")
    } else if has_month {
        format!("{year}/{month:02}")
    } else {
        String::new()
    };
    let dotted = if has_day {
        format!("{year}.{month:02}.{day:02}")
    } else if has_month {
        format!("{year}.{month:02}")
    } else {
        String::new()
    };
    // 公元前用 BC 标注，不写负号
    let (bce_numeric, bce_plain) = if negative {
        (
            format!("{numeric} BC"),
            format!(
                "公元前{}年{}{}",
                year.unsigned_abs(),
                if has_month {
                    format!("{month}月")
                } else {
                    String::new()
                },
                if has_day {
                    format!("{day}日")
                } else {
                    String::new()
                }
            ),
        )
    } else {
        (
            numeric.clone(),
            format!(
                "{}{year}年{}{}",
                era_prefix(year),
                if has_month {
                    format!("{month}月")
                } else {
                    String::new()
                },
                if has_day {
                    format!("{day}日")
                } else {
                    String::new()
                }
            ),
        )
    };
    // 「前139年2月」这种短写法：跟 `middle` 一样按段数，只是年份用阿拉伯数字
    let short = format!(
        "{}年{}{}",
        era_year(year),
        if has_month {
            format!("{month}月")
        } else {
            String::new()
        },
        if has_day {
            format!("{day}日")
        } else {
            String::new()
        }
    );
    let mut forms = vec![middle, short, bce_numeric, bce_plain];
    // 带斜杠与点的写法也进候选（公元前同样标 BC）
    if !negative {
        if !slashed.is_empty() {
            forms.push(slashed);
        }
        if !dotted.is_empty() {
            forms.push(dotted);
        }
    }
    // 与 `rq` 同一档规格再补一层：星期、紧凑数字、欧式与美式、ISO 周。
    // 位置卡在几种年月日写法之后。
    // 下面这些只有公元后才成立：两位年、ISO 周、欧式 / 美式都假定正数年份
    if !negative {
        if has_day {
            let weekday = weekday_name(date.weekday());
            let weekday_long = weekday_long_name(date.weekday());
            let iso = date.iso_week_date();
            forms.extend([
                format!("{year}年{month}月{day}日 {weekday_long}"),
                format!("{year}-{month:02}-{day:02} {weekday}"),
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
            ]);
        } else if has_month {
            forms.push(format!("{:02}-{month:02}", year % 100));
        }
    }
    forms.dedup();
    Some(forms)
}
/// 年份的读法：正数原样，负数写成「前221」。
fn era_year(year: i16) -> String {
    if year < 0 {
        format!("前{}", year.unsigned_abs())
    } else {
        year.to_string()
    }
}

/// 纪元前缀：公元 / 公元前。
fn era_prefix(year: i16) -> &'static str {
    if year < 0 { "公元前" } else { "公元" }
}

/// 年份逐位读：`1998` → 一九九八，`2006` → 二〇〇六（年份里的零按日期惯例写「〇」）。
fn chinese_year(year: i16) -> String {
    const DIGITS: [&str; 10] = ["〇", "一", "二", "三", "四", "五", "六", "七", "八", "九"];
    // 公元前：负号不进逐位读法，写成「前二二一」
    if year < 0 {
        return format!("前{}", chinese_year(-year));
    }
    year.to_string()
        .bytes()
        .map(|b| DIGITS[usize::from(b.wrapping_sub(b'0'))])
        .collect()
}

/// `rq`：今天。中文与数字两套、带星期、月日、纯数字串、两位年、欧式与美式、ISO 周。
pub fn date_forms(now: &Zoned) -> Vec<String> {
    let (year, month, day) = (now.year(), now.month(), now.day());
    let weekday = weekday_name(now.weekday());
    let weekday_long = weekday_long_name(now.weekday());
    // ISO 8601 的周号：`2026-W44-7`
    let iso = now.date().iso_week_date();
    let mut forms = vec![
        format!("{year}年{month}月{day}日"),
        format!("{year}-{month:02}-{day:02}"),
        format!("{year}/{month:02}/{day:02}"),
    ];
    forms.extend([
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
    ]);
    forms
}

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
        // Unix 秒 / 毫秒不在这里给：要时间戳就敲 `vts`（见 [`timestamp_forms`]）。
        // 混在钟点候选里既挡路，又会让 `looks_like_timestamp` 把整组当成时间型反复重算。
    ]
}

/// 表达式键之后是这个词时出时间戳（`vts`）。
pub const TIMESTAMP_KEYWORD: &str = "ts";

/// `vts`：Unix 秒、UTC（RFC 3339，`2026-09-03T11:06:23.000Z`）、Unix 毫秒、
/// 带时区的 ISO 8601（`2026-09-03T19:06:23.000+08:00`）。
pub fn timestamp_forms(now: &Zoned) -> Vec<String> {
    let instant = now.timestamp();
    // ISO 那两条带毫秒：与 `sj` 的毫秒档对齐，也免得「整秒」看起来像少了这一档
    let millis = instant.subsec_nanosecond() / 1_000_000;
    vec![
        instant.as_second().to_string(),
        format!(
            "{}.{millis:03}Z",
            now.with_time_zone(TimeZone::UTC)
                .strftime("%Y-%m-%dT%H:%M:%S")
        ),
        instant.as_millisecond().to_string(),
        format!(
            "{}.{millis:03}{}",
            now.strftime("%Y-%m-%dT%H:%M:%S"),
            now.strftime("%:z")
        ),
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

    /// 固定的北京时间：2026-11-01 17:07:25.123。
    ///
    /// 用固定偏移而不是 `[Asia/Shanghai]`：CI 的 Linux 容器里没有 tzdata，
    /// 按名字解析时区会失败（`no time zone database configured`）。
    fn fixed() -> Zoned {
        jiff::civil::date(2026, 11, 1)
            .at(17, 7, 25, 123_000_000)
            .to_zoned(jiff::tz::TimeZone::fixed(jiff::tz::offset(8)))
            .expect("时刻常量合法")
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
        // Unix 秒 / 毫秒不再混在 `sj` 里：要时间戳敲 `vts`
        assert!(
            !forms
                .iter()
                .any(|f| matches!(f.len(), 10 | 13) && f.bytes().all(|b| b.is_ascii_digit())),
            "`sj` 不该再给 Unix 时间戳：{forms:?}"
        );
    }

    #[test]
    fn weekday_has_three_spellings() {
        let forms = weekday_forms(&fixed());
        assert_eq!(forms, vec!["星期日", "周日", "礼拜日"]);
    }
}
