//! 统计页的数：输入量（`usage-hourly.tsv`，按天、按小时）、词汇（`user-vocab.tsv`，有等级表时按级分）与云服务的请求和 Token。
//! 输入法记、这里只读，进统计页时读一次。还有几种给人看的写法：千位分隔、万 / 亿字、跟一本书比。

mod cloud;
mod input;
mod numbers;

pub use cloud::CloudStats;
pub use input::{InputStats, clean_rate, first_choice_rate, keys_per_hanzi};
pub use numbers::{group_digits, hanzi_text, scale_line, tokens_text};

use std::path::PathBuf;

use jiff::civil::Date;
use qingjian_core::{Language, Usage, VocabularySummary};
use qingjian_learning::{UsageStats, VocabularyBook};
use qingjian_platform::GeneralConfig;
use qingjian_predict::{API_USAGE_FILE, ApiUsageHours, ApiUsageStats};
use qingjian_translate::LevelTable;

use crate::system::paths;

pub struct Stats {
    pub input: InputStats,

    /// 学习语言的名字与词汇汇总；学习语言关着为 `None`。
    pub vocabulary: Option<(&'static str, VocabularySummary)>,

    pub cloud: CloudStats,
}

impl Stats {
    pub fn load(general: &GeneralConfig) -> Self {
        let now = jiff::Zoned::now().datetime();
        let today = now.date();
        let dir = paths::user_dir().unwrap_or_default();
        let hour = now.hour() as u8;
        let yesterday = today.yesterday().unwrap_or(today);
        let usage = UsageStats::open(&dir);
        let input = InputStats {
            today,
            hour,
            days: usage.days(),
            hours: last_24(usage.hours_on(yesterday), usage.hours_on(today), hour),
        };
        let vocabulary = (!general.learning_language_off()).then(|| {
            let language = general
                .learning_language
                .parse::<Language>()
                .unwrap_or(Language::English);
            let mut book = VocabularyBook::open(dir.join("user-vocab.tsv"));
            if let Some(path) = level_table(language)
                && let Ok(table) = LevelTable::from_path(&path)
            {
                book = book.with_levels(language, table);
            }
            (language_name(language), book.summary_on(language, today))
        });
        let totals = ApiUsageStats::load(&dir.join(API_USAGE_FILE))
            .inspect_err(|error| tracing::warn!(%error, "API 用量统计读取失败"))
            .ok();
        let hours = ApiUsageHours::load(&dir)
            .inspect_err(|error| tracing::warn!(%error, "按小时的 API 用量读取失败"))
            .unwrap_or_default();
        let cloud = CloudStats {
            today,
            hour,
            totals,
            days: hours.days(),
            hours: last_24(hours.hours_on(yesterday), hours.hours_on(today), hour),
        };
        Self {
            input,
            vocabulary,
            cloud,
        }
    }
}

/// `from` 到 `to`（含两头）这几天加起来。
fn sum_days<T: Copy + Default + std::ops::AddAssign>(
    days: &std::collections::BTreeMap<Date, T>,
    from: Date,
    to: Date,
) -> T {
    let mut total = T::default();
    for value in days.range(from..=to).map(|(_, value)| *value) {
        total += value;
    }
    total
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

/// 这一周的星期日（一周从周日算，同代码托管网站的提交日历）。
pub fn week_start(date: Date) -> Date {
    let back = i64::from(date.weekday().to_sunday_zero_offset());
    date.saturating_sub(jiff::Span::new().days(back))
}

/// 这个月的 1 号。
fn first_of_month(date: Date) -> Date {
    date.first_of_month()
}

/// 速度（字 / 分钟）：记了时长的那些上屏打出的汉字除以打字时长，老记录与原样上屏只有字数、不在里面。
/// 打字不到 `min_ms` 不算，免得几个字就出个离谱的数。
fn speed(usage: &Usage, min_ms: u64) -> Option<f64> {
    (usage.typing_ms >= min_ms)
        .then(|| usage.typed_hanzi as f64 * 60_000.0 / usage.typing_ms as f64)
}

fn language_name(language: Language) -> &'static str {
    match language {
        Language::Chinese => "中文",
        Language::English => "英语",
        Language::Japanese => "日语",
        Language::Spanish => "西班牙语",
        Language::Vietnamese => "越南语",
        Language::German => "德语",
    }
}

/// 词汇等级表：macOS 在 `Qingjian.app/Contents/Resources/`，其他平台在随包根的 `assets/levels/`。
fn level_table(language: Language) -> Option<PathBuf> {
    let name = format!("levels-{}.tsv", language.code());
    let found = crate::system::paths::mac_resources()
        .into_iter()
        .map(|dir| dir.join(&name))
        .find(|path| path.is_file());
    if found.is_some() {
        return found;
    }
    crate::system::paths::bundled(&format!("assets/levels/{name}"))
}

#[cfg(test)]
mod tests {
    use super::last_24;

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
}
