//! 统计页的数：输入量（`usage-hourly.tsv`，按天、按小时）、词汇（`user-vocab.tsv`，有等级表时按级分）与云服务的请求和 Token。
//! 输入法记、这里只读，进统计页时读一次。还有几种给人看的写法：千位分隔、万 / 亿字、跟一本书比。

mod cloud;
mod input;
mod numbers;

pub use cloud::CloudStats;
pub use input::InputStats;
pub use numbers::{group_digits, hanzi_text, scale_line, tokens_text};

use std::path::PathBuf;

use qingjian_core::{Language, VocabularySummary};
use qingjian_learning::figures::last_24;
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

pub fn language_name(language: Language) -> &'static str {
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
