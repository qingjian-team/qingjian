use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use jiff::civil::Date;
use qingjian_core::storage::{read_text_lossy, write_atomic_str};
use qingjian_core::{Usage, UsageMeter, UsageSummary};

use crate::error::LearningError;

/// 汇总里「最近」算几天（含今天）。
const RECENT_DAYS: i64 = 7;

/// 输入统计的落盘：按天一行 `日期\t汉字\t中文词\t英文词\t上屏次数`，只写在本机数据目录里。
///
/// 与输入日志无关：日志关着或清空了统计照记；文件坏了按行跳过，读不了就只在内存里数（记一次警告），输入不受影响。
#[derive(Debug, Default)]
pub struct UsageStats {
    /// 每天的用量，按日期排好。
    days: BTreeMap<Date, Usage>,

    /// 自上次保存后有没有新记录。
    dirty: bool,

    /// [`UsageMeter::flush`] 时写回的路径；`None` 只在内存里数。
    path: Option<PathBuf>,
}

impl UsageStats {
    /// 从文件加载（不存在就从零开始，flush 时建）；读不了就退回只在内存里数。
    pub fn open(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        match read_text_lossy(&path) {
            Ok(text) => Self {
                days: text.as_deref().map(parse).unwrap_or_default(),
                dirty: false,
                path: Some(path),
            },
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "输入统计读不了，本次只在内存里数");
                Self::default()
            }
        }
    }

    /// 有记录的天数。
    pub fn len(&self) -> usize {
        self.days.len()
    }

    pub fn is_empty(&self) -> bool {
        self.days.is_empty()
    }

    /// 记到某一天上（[`UsageMeter::record`] 记到今天）。
    pub fn record_on(&mut self, date: Date, usage: Usage) {
        if usage.is_empty() {
            return;
        }
        *self.days.entry(date).or_default() += usage;
        self.dirty = true;
    }

    /// 以 `today` 为准的汇总（[`UsageMeter::summary`] 以本机今天为准）。
    pub fn summary_on(&self, today: Date) -> UsageSummary {
        let recent_from = today.saturating_sub(jiff::Span::new().days(RECENT_DAYS - 1));
        let mut summary = UsageSummary {
            since: self.days.keys().next().map(ToString::to_string),
            days: self.days.len() as u32,
            ..UsageSummary::default()
        };
        for (date, usage) in &self.days {
            summary.total += *usage;
            if *date >= recent_from && *date <= today {
                summary.week += *usage;
            }
            if *date == today {
                summary.today = *usage;
            }
        }
        summary
    }

    /// 写回文件（没有新记录就什么都不做）。
    pub fn save(&mut self) -> Result<(), LearningError> {
        let Some(path) = self.path.clone() else {
            return Ok(());
        };
        if !self.dirty {
            return Ok(());
        }
        self.save_to(&path)?;
        self.dirty = false;
        Ok(())
    }

    fn save_to(&self, path: &Path) -> Result<(), LearningError> {
        let mut text = String::from("# 日期\t汉字\t中文词\t英文词\t上屏次数\n");
        for (date, usage) in &self.days {
            let _ = writeln!(
                text,
                "{date}\t{}\t{}\t{}\t{}",
                usage.hanzi, usage.words, usage.english_words, usage.commits
            );
        }
        write_atomic_str(path, &text)?;
        Ok(())
    }
}

/// 按行解析；格式不对的行跳过（记警告），同一天出现两次就加起来。
fn parse(text: &str) -> BTreeMap<Date, Usage> {
    let mut days = BTreeMap::new();
    for (number, line) in text.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match parse_line(line) {
            Some((date, usage)) => *days.entry(date).or_default() += usage,
            None => tracing::warn!(line = number + 1, "输入统计有坏行，跳过"),
        }
    }
    days
}

fn parse_line(line: &str) -> Option<(Date, Usage)> {
    let mut fields = line.split('\t');
    let date: Date = fields.next()?.parse().ok()?;
    let mut number = || fields.next()?.parse::<u64>().ok();
    let usage = Usage {
        hanzi: number()?,
        words: number()?,
        english_words: number()?,
        commits: number()?,
    };
    Some((date, usage))
}

fn today() -> Date {
    jiff::Zoned::now().date()
}

impl UsageMeter for UsageStats {
    fn record(&mut self, usage: Usage) {
        self.record_on(today(), usage);
    }

    fn flush(&mut self) {
        if let Err(error) = self.save() {
            tracing::warn!(%error, "输入统计保存失败");
        }
    }

    fn summary(&self) -> UsageSummary {
        self.summary_on(today())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(text: &str) -> Date {
        text.parse().unwrap()
    }

    fn usage(hanzi: u64) -> Usage {
        Usage {
            hanzi,
            words: 1,
            english_words: 0,
            commits: 1,
        }
    }

    #[test]
    fn summarizes_today_recent_week_and_total() {
        let mut stats = UsageStats::default();
        stats.record_on(date("2026-08-01"), usage(100));
        stats.record_on(date("2026-08-31"), usage(10));
        stats.record_on(date("2026-09-01"), usage(20));
        stats.record_on(date("2026-09-06"), usage(3));
        stats.record_on(date("2026-09-06"), usage(4));
        stats.record_on(date("2026-09-06"), Usage::default());
        let summary = stats.summary_on(date("2026-09-06"));
        assert_eq!(summary.today.hanzi, 7);
        assert_eq!(summary.today.commits, 2);
        // 最近 7 天：8-31 到 9-6，8-31 恰好在内
        assert_eq!(summary.week.hanzi, 37);
        assert_eq!(summary.total.hanzi, 137);
        assert_eq!(summary.days, 4);
        assert_eq!(summary.since.as_deref(), Some("2026-08-01"));
        assert!(
            UsageStats::default()
                .summary_on(date("2026-09-06"))
                .since
                .is_none()
        );
    }

    #[test]
    fn round_trips_through_the_file_and_skips_bad_lines() {
        let dir = std::env::temp_dir().join(format!("qingjian-usage-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("usage.tsv");
        std::fs::write(
            &path,
            "# 头\n2026-09-05\t12\t3\t1\t4\n坏行\n2026-09-05\t1\t1\t0\t1\n",
        )
        .unwrap();
        let mut stats = UsageStats::open(&path);
        assert_eq!(stats.len(), 1);
        assert_eq!(stats.summary_on(date("2026-09-05")).today.hanzi, 13);
        // 没新记录不写
        stats.save().unwrap();
        stats.record_on(date("2026-09-06"), usage(5));
        stats.save().unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            text,
            "# 日期\t汉字\t中文词\t英文词\t上屏次数\n2026-09-05\t13\t4\t1\t5\n2026-09-06\t5\t1\t0\t1\n"
        );
        let reloaded = UsageStats::open(&path);
        assert_eq!(reloaded.summary_on(date("2026-09-06")).total.hanzi, 18);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
