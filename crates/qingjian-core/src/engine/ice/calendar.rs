//! 雾凇全拼的日期格式和农历，使用本地民用日期。

use jiff::Zoned;
use lunar_rust::{
    lunar::LunarRefHelper,
    solar::{self, SolarRefHelper},
};

use super::profile::Profile;

pub(super) fn forms(kind: &str, now: &Zoned) -> Vec<String> {
    let (y, m, d) = (now.year(), now.month(), now.day());
    let (h, min, s) = (now.hour(), now.minute(), now.second());
    match kind {
        "date" => vec![
            format!("{y}-{m:02}-{d:02}"),
            format!("{y}/{m:02}/{d:02}"),
            format!("{y}.{m:02}.{d:02}"),
            format!("{y}{m:02}{d:02}"),
            format!("{y}年{m}月{d}日"),
        ],
        "time" => {
            let period = match h {
                5..=10 => "早上",
                11..=12 => "中午",
                13..=17 => "下午",
                18..=23 => "晚上",
                _ => "凌晨",
            };
            let hour = if h % 12 == 0 { 12 } else { h % 12 };
            vec![
                format!("{h:02}:{min:02}"),
                format!("{h:02}:{min:02}:{s:02}"),
                format!("{period} {hour:02}:{min:02}"),
                format!("{hour:02}:{min:02} {}", if h < 12 { "AM" } else { "PM" }),
            ]
        }
        "week" => {
            let short = crate::shortcut::weekday_forms(now)[0]
                .trim_start_matches("星期")
                .to_owned();
            vec![
                format!("星期{short}"),
                format!("礼拜{short}"),
                format!("周{short}"),
            ]
        }
        "datetime" => {
            let seconds = now.offset().seconds();
            let tz = if seconds == 0 {
                "Z".to_owned()
            } else {
                format!(
                    "{}{:02}:{:02}",
                    if seconds >= 0 { '+' } else { '-' },
                    seconds.abs() / 3600,
                    seconds.abs() / 60 % 60
                )
            };
            vec![
                format!("{y}-{m:02}-{d:02}T{h:02}:{min:02}:{s:02}{tz}"),
                format!("{y}-{m:02}-{d:02} {h:02}:{min:02}:{s:02}"),
                format!("{y}{m:02}{d:02}{h:02}{min:02}{s:02}"),
            ]
        }
        "timestamp" => vec![now.timestamp().as_second().to_string()],
        "datezh" => {
            let year: String = y
                .to_string()
                .chars()
                .map(|c| {
                    "〇一二三四五六七八九"
                        .chars()
                        .nth(c.to_digit(10).unwrap_or(0) as usize)
                        .unwrap_or(c)
                })
                .collect();
            let suffix = format!(
                "年{}月{}日",
                crate::shortcut::chinese_lower(&m.to_string()),
                crate::shortcut::chinese_lower(&d.to_string())
            );
            vec![
                format!("{year}{suffix}"),
                format!("{}{suffix}", year.replace('〇', "零")),
                format!("{y}年{m}月{d}日"),
            ]
        }
        "dateen" => {
            const MONTHS: [&str; 12] = [
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December",
            ];
            let month = MONTHS[(m - 1) as usize];
            vec![format!("{d} {month} {y}"), format!("{month} {d}, {y}")]
        }
        _ => Vec::new(),
    }
}

impl Profile {
    pub(super) fn lunar(&self, date: jiff::civil::Date) -> (String, String) {
        if !(1900..=2100).contains(&date.year()) {
            return ("错误".to_owned(), "仅支持1900-2100之间的日期".to_owned());
        }
        let lunar = solar::from_ymd(
            i64::from(date.year()),
            i64::from(date.month()),
            i64::from(date.day()),
        )
        .get_lunar();
        let month = format!("{}月", lunar.get_month_in_chinese().replace("冬", "十一"));
        let day = lunar.get_day_in_chinese();
        let term = lunar.get_jie_qi();
        let weekday = format!(
            "星期{}",
            ["日", "一", "二", "三", "四", "五", "六"]
                [date.weekday().to_sunday_zero_offset() as usize]
        );
        let values = [
            ("干支年", lunar.get_year_in_gan_zhi()),
            ("生肖", lunar.get_year_sheng_xiao()),
            ("农历月", month.replace("腊", "十二")),
            ("俗称农历月", month),
            ("农历日", day.replace("廿", "二十")),
            ("简记农历日", day),
            ("星期", weekday.clone()),
            ("节气", term.clone()),
        ];
        let mut text = self.lunar_template.clone();
        for (key, value) in values {
            text = text.replace(&format!("{{{key}}}"), &value);
        }
        let comment = if term.is_empty() {
            weekday
        } else {
            format!("{weekday} {term}")
        };
        (text, comment)
    }
}
