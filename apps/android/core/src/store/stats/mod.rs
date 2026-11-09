//! App 的输入统计：「我的」页的几个数（[`stats`]），首页统计看板与统计详情页（[`stats_detail`]）。
//! 派生数一律用 [`qingjian_learning::figures`]，与 PC 设置程序同一套口径。直接读学习数据目录里的输入统计（输入法每次上屏都当场追加进文件，App 这边读到的就是最新的）。

mod count;
mod day;
mod detail;
mod level;
mod summary;
mod vocabulary;

pub use count::StatsCount;
pub use day::StatsDay;
pub use detail::StatsDetail;
pub use level::StatsLevel;
pub use summary::StatsSummary;
pub use vocabulary::StatsVocabulary;

use std::path::Path;

use qingjian_core::UsageMeter;
use qingjian_learning::figures::{
    AVERAGE_MIN_MS, TODAY_MIN_MS, active_days, best_day, clean_rate, daily_average,
    first_choice_rate, keys_per_hanzi, last_24, month, recent, speed, total,
};
use qingjian_learning::{UsageStats, VocabularyBook};
use qingjian_translate::LevelTable;

use crate::options::learning_language;

/// 柱状图列最近几天。
const CHART_DAYS: i64 = 30;

pub fn stats(user_dir: &Path) -> StatsSummary {
    let summary = UsageStats::open(user_dir).summary();
    StatsSummary {
        today: summary.today.into(),
        recent: summary.week.into(),
        total: summary.total.into(),
        days: summary.days,
    }
}

pub(crate) fn stats_json(user_dir: &Path) -> String {
    serde_json::to_string(&stats(user_dir)).expect("统计总能序列化")
}

/// 统计详情；`language` 是学习语言的代码（`off` 不出词汇一节），等级表在随包数据目录。
pub fn stats_detail(data_dir: &Path, user_dir: &Path, language: &str) -> StatsDetail {
    let now = jiff::Zoned::now().datetime();
    let today = now.date();
    let hour = now.hour() as u8;
    let usage = UsageStats::open(user_dir);
    let days = usage.days();
    let yesterday = today.yesterday().unwrap_or(today);
    let hours = last_24(usage.hours_on(yesterday), usage.hours_on(today), hour);
    let chart_from = today.saturating_sub(jiff::Span::new().days(CHART_DAYS - 1));
    let chart = std::iter::successors(Some(chart_from), |date| {
        date.tomorrow().ok().filter(|next| *next <= today)
    })
    .map(|date| StatsDay {
        date: date.to_string(),
        hanzi: days.get(&date).map_or(0, |day| day.hanzi),
    })
    .collect();
    let total = total(&days);
    let today_usage = days.get(&today).copied().unwrap_or_default();
    StatsDetail {
        today: today_usage.into(),
        recent: recent(&days, today).into(),
        month: month(&days, today).into(),
        total: total.into(),
        hours: hours.iter().map(|usage| usage.hanzi).collect(),
        hour,
        days: chart,
        best_day: best_day(&days).map(|(date, day)| StatsDay {
            date: date.to_string(),
            hanzi: day.hanzi,
        }),
        active_days: active_days(&days) as u64,
        average: daily_average(&days).unwrap_or(0),
        today_speed: speed(&today_usage, TODAY_MIN_MS),
        average_speed: speed(&total, AVERAGE_MIN_MS),
        first_choice_rate: first_choice_rate(&total),
        clean_rate: clean_rate(&total),
        keys_per_hanzi: keys_per_hanzi(&total),
        vocabulary: learning_language(language).map(|language| {
            let mut book = VocabularyBook::open(user_dir.join("user-vocab.tsv"));
            let levels = data_dir.join(format!("levels-{}.tsv", language.code()));
            if let Ok(table) = LevelTable::from_path(&levels) {
                book = book.with_levels(language, table);
            }
            StatsVocabulary::new(language.code(), book.summary_on(language, today))
        }),
    }
}

pub(crate) fn stats_detail_json(data_dir: &Path, user_dir: &Path, language: &str) -> String {
    serde_json::to_string(&stats_detail(data_dir, user_dir, language)).expect("统计总能序列化")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_directory_counts_zero() {
        let dir = std::env::temp_dir().join(format!("qingjian-stats-{}", std::process::id()));
        let summary = stats(&dir);
        assert_eq!(summary.today.hanzi, 0);
        assert_eq!(summary.days, 0);
    }

    #[test]
    fn detail_has_a_full_chart_even_when_empty() {
        let dir = std::env::temp_dir().join(format!("qingjian-detail-{}", std::process::id()));
        let detail = stats_detail(&dir, &dir, "en");
        assert_eq!(detail.hours.len(), 24);
        assert_eq!(detail.days.len(), CHART_DAYS as usize);
        assert!(detail.best_day.is_none());
        assert_eq!(detail.vocabulary.unwrap().seen, 0);
        assert!(stats_detail(&dir, &dir, "off").vocabulary.is_none());
    }
}
