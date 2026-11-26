//! 节气与节日快捷候选，按距今天的天数排序。

use jiff::civil::{Date, Weekday};
use tyme4rs::tyme::lunar::LunarDay;
use tyme4rs::tyme::solar::{SolarDay, SolarTerm};
use tyme4rs::tyme::{Culture, Tyme};

fn solar_date(day: SolarDay) -> Date {
    Date::new(
        day.get_year() as i16,
        day.get_month() as i8,
        day.get_day() as i8,
    )
    .expect("现代历法日期合法")
}

fn label(name: &str, date: Date, today: Date) -> String {
    let days = today
        .until((jiff::Unit::Day, date))
        .expect("日期差合法")
        .get_days();
    if days == 0 {
        format!("{name} {date}（今天）")
    } else {
        format!("{name} {date}（还有{days}天）")
    }
}

pub(super) fn solar_terms(today: Date) -> Vec<String> {
    if !(1901..2100).contains(&today.year()) {
        return Vec::new();
    }
    let solar = SolarDay::from_ymd(
        today.year().into(),
        today.month() as usize,
        today.day() as usize,
    );
    let mut term = solar.get_term();
    if solar_date(term.get_solar_day()) < today {
        term = term.next(1);
    }
    (0..24)
        .map(|i| {
            let term = term.next(i);
            label(&term.get_name(), solar_date(term.get_solar_day()), today)
        })
        .collect()
}

pub(super) fn festivals(today: Date) -> Vec<String> {
    if !(1901..2100).contains(&today.year()) {
        return Vec::new();
    }
    let mut events = Vec::new();
    for year in today.year() - 1..=today.year() + 1 {
        let y = isize::from(year);
        for (name, m, d) in [
            ("元旦", 1, 1),
            ("情人节", 2, 14),
            ("妇女节", 3, 8),
            ("劳动节", 5, 1),
            ("青年节", 5, 4),
            ("儿童节", 6, 1),
            ("教师节", 9, 10),
            ("国庆节", 10, 1),
            ("平安夜", 12, 24),
            ("圣诞节", 12, 25),
        ] {
            events.push((Date::new(year, m, d).unwrap(), name));
        }
        for (name, m, weekday, nth) in [
            ("母亲节", 5, Weekday::Sunday, 2),
            ("父亲节", 6, Weekday::Sunday, 3),
            ("感恩节", 11, Weekday::Thursday, 4),
        ] {
            let first = Date::new(year, m, 1).unwrap();
            events.push((first.nth_weekday(nth, weekday).unwrap(), name));
        }
        for (name, m, d) in [
            ("春节", 1, 1),
            ("元宵节", 1, 15),
            ("端午节", 5, 5),
            ("七夕", 7, 7),
            ("中元节", 7, 15),
            ("中秋节", 8, 15),
            ("重阳节", 9, 9),
            ("腊八节", 12, 8),
        ] {
            events.push((
                solar_date(LunarDay::from_ymd(y, m, d).get_solar_day()),
                name,
            ));
        }
        let new_year = LunarDay::from_ymd(y, 1, 1).get_solar_day();
        events.push((solar_date(new_year.next(-1)), "除夕"));
        events.push((
            solar_date(SolarTerm::from_name(y, "清明").get_solar_day()),
            "清明节",
        ));
    }
    events.sort_by_key(|&(date, _)| date);
    let mut seen = std::collections::HashSet::new();
    events
        .into_iter()
        .filter(|&(date, name)| date >= today && seen.insert(name))
        .map(|(date, name)| label(name, date, today))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn occasions_cover_today_and_cross_the_new_year() {
        let terms = solar_terms(Date::constant(2026, 12, 22));
        assert_eq!(terms.len(), 24);
        assert_eq!(terms[0], "冬至 2026-12-22（今天）");
        assert!(terms[1].starts_with("小寒 2027-01-"));
        let days = festivals(Date::constant(2026, 11, 20));
        assert_eq!(days[0], "感恩节 2026-11-26（还有6天）");
        assert!(days.iter().any(|s| s.starts_with("春节 2027-02-06")));
        assert!(days.iter().any(|s| s.starts_with("除夕 2027-02-05")));
    }
}
