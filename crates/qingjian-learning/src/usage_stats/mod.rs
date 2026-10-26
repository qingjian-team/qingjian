//! 输入统计的落盘：按小时累计，每次上屏当场往 `usage-hourly.tsv` 末尾追加一行，进程被杀、关机也只丢正在写的那一行；
//! 追加多了再整理成一个时段一行（先写临时文件再换名）。只写在本机数据目录里。
//!
//! 与输入日志无关：日志关着或清空了统计照记。0.2 以前按天记的 `usage.tsv` 照读不写，合进按天的数里。

pub mod figures;
mod file;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use jiff::civil::{Date, DateTime};
use qingjian_core::storage::{read_text_lossy, write_atomic_str};
use qingjian_core::{Usage, UsageMeter, UsageSummary};

use crate::error::LearningError;

/// 按小时记的统计文件。
pub const USAGE_FILE: &str = "usage-hourly.tsv";

/// 0.2 以前按天记的统计文件，只读。
const LEGACY_USAGE_FILE: &str = "usage.tsv";

/// 文件行数比时段数多出这么多就整理一次。
const COMPACT_AFTER: usize = 2_000;

#[derive(Debug, Default)]
pub struct UsageStats {
    /// 按小时的用量，键是日期与时（0–23）。
    hours: BTreeMap<(Date, u8), Usage>,

    /// 按天的老记录，没有时段。
    legacy: BTreeMap<Date, Usage>,

    /// 按小时的文件；`None` 只在内存里数。
    path: Option<PathBuf>,

    /// 文件里有多少行记录（整理后等于时段数）。
    lines: usize,

    /// 有记录没追加进去（写失败），下次落盘整个重写。
    rewrite: bool,
}

impl UsageStats {
    /// 读数据目录里的统计（不存在就从零开始，第一次上屏时建）；读不了就退回只在内存里数。
    pub fn open(dir: &Path) -> Self {
        let legacy = match read_text_lossy(&dir.join(LEGACY_USAGE_FILE)) {
            Ok(text) => text.as_deref().map(file::parse_days).unwrap_or_default(),
            Err(error) => {
                tracing::warn!(%error, "按天的老输入统计读不了，跳过");
                BTreeMap::new()
            }
        };
        let path = dir.join(USAGE_FILE);
        match read_text_lossy(&path) {
            Ok(text) => {
                let (hours, lines) = text.as_deref().map(file::parse_hours).unwrap_or_default();
                Self {
                    hours,
                    legacy,
                    path: Some(path),
                    lines,
                    rewrite: false,
                }
            }
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "输入统计读不了，本次只在内存里数");
                Self {
                    legacy,
                    ..Self::default()
                }
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.hours.is_empty() && self.legacy.is_empty()
    }

    /// 记到某个时刻所在的那个小时（[`UsageMeter::record`] 记到现在），当场追加进文件。
    pub fn record_at(&mut self, at: DateTime, usage: Usage) {
        if usage.is_empty() {
            return;
        }
        let key = (at.date(), at.hour() as u8);
        *self.hours.entry(key).or_default() += usage;
        let Some(path) = &self.path else {
            return;
        };
        if self.rewrite {
            return;
        }
        match file::append(path, &file::hour_line(key.0, key.1, &usage)) {
            Ok(()) => self.lines += 1,
            Err(error) => {
                tracing::warn!(%error, "输入统计追加失败，下次落盘整个重写");
                self.rewrite = true;
            }
        }
    }

    /// 每天的用量（老的按天记录与按小时的合起来），按日期排好。
    pub fn days(&self) -> BTreeMap<Date, Usage> {
        let mut days = self.legacy.clone();
        for ((date, _), usage) in &self.hours {
            *days.entry(*date).or_default() += *usage;
        }
        days
    }

    /// 某天 24 个小时各自的用量；老的按天记录不分时段，不在里面。
    pub fn hours_on(&self, date: Date) -> [Usage; 24] {
        let mut hours = [Usage::default(); 24];
        for ((_, hour), usage) in self.hours.range((date, 0)..=(date, 23)) {
            hours[*hour as usize] = *usage;
        }
        hours
    }

    /// 以 `today` 为准的汇总（[`UsageMeter::summary`] 以本机今天为准）。
    pub fn summary_on(&self, today: Date) -> UsageSummary {
        let days = self.days();
        let recent_from = figures::recent_start(today);
        let mut summary = UsageSummary {
            since: days.keys().next().map(ToString::to_string),
            days: days.len() as u32,
            ..UsageSummary::default()
        };
        for (date, usage) in &days {
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

    /// 追加失败过、或文件里重复的时段太多时，整理成一个时段一行重写；平时什么都不做。
    pub fn save(&mut self) -> Result<(), LearningError> {
        let Some(path) = self.path.clone() else {
            return Ok(());
        };
        if !self.rewrite && self.lines < self.hours.len() + COMPACT_AFTER {
            return Ok(());
        }
        let mut text = String::from(file::HEADER);
        for ((date, hour), usage) in &self.hours {
            text.push_str(&file::hour_line(*date, *hour, usage));
        }
        write_atomic_str(&path, &text)?;
        self.lines = self.hours.len();
        self.rewrite = false;
        Ok(())
    }
}

impl UsageMeter for UsageStats {
    fn record(&mut self, usage: Usage) {
        self.record_at(jiff::Zoned::now().datetime(), usage);
    }

    fn flush(&mut self) {
        if let Err(error) = self.save() {
            tracing::warn!(%error, "输入统计保存失败");
        }
    }

    fn summary(&self) -> UsageSummary {
        self.summary_on(jiff::Zoned::now().date())
    }
}
