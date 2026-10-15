use std::path::PathBuf;

use jiff::civil::{Date, DateTime};
use qingjian_core::Usage;

use super::{COMPACT_AFTER, USAGE_FILE, UsageStats};

fn date(text: &str) -> Date {
    text.parse().unwrap()
}

fn at(text: &str) -> DateTime {
    text.parse().unwrap()
}

fn usage(hanzi: u64) -> Usage {
    Usage {
        hanzi,
        words: 1,
        commits: 1,
        choices: 1,
        first_choices: 1,
        typed_hanzi: hanzi,
        keys: 4,
        typing_ms: 900,
        ..Usage::default()
    }
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qingjian-usage-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn summarizes_today_recent_week_and_total() {
    let mut stats = UsageStats::default();
    stats.record_at(at("2026-08-01T09:00"), usage(100));
    stats.record_at(at("2026-08-31T10:00"), usage(10));
    stats.record_at(at("2026-09-01T10:00"), usage(20));
    stats.record_at(at("2026-09-06T08:10"), usage(3));
    stats.record_at(at("2026-09-06T21:59"), usage(4));
    stats.record_at(at("2026-09-06T21:00"), Usage::default());
    let summary = stats.summary_on(date("2026-09-06"));
    assert_eq!(summary.today.hanzi, 7);
    assert_eq!(summary.today.commits, 2);
    // 最近 7 天：8-31 到 9-6，8-31 恰好在内
    assert_eq!(summary.week.hanzi, 37);
    assert_eq!(summary.total.hanzi, 137);
    assert_eq!(summary.days, 4);
    assert_eq!(summary.since.as_deref(), Some("2026-08-01"));
    let hours = stats.hours_on(date("2026-09-06"));
    assert_eq!(hours[8].hanzi, 3);
    assert_eq!(hours[21].hanzi, 4);
    assert_eq!(hours[9], Usage::default());
}

#[test]
fn every_commit_is_on_disk_before_any_flush() {
    let dir = scratch("append");
    let mut stats = UsageStats::open(&dir);
    stats.record_at(at("2026-09-06T08:10"), usage(3));
    stats.record_at(at("2026-09-06T08:40"), usage(4));
    // 不落盘、直接扔掉：像进程被杀
    drop(stats);
    let reopened = UsageStats::open(&dir);
    assert_eq!(reopened.hours_on(date("2026-09-06"))[8].hanzi, 7);
    assert_eq!(reopened.hours_on(date("2026-09-06"))[8].keys, 8);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn reads_the_old_daily_file_and_skips_torn_lines() {
    let dir = scratch("legacy");
    std::fs::write(
        dir.join("usage.tsv"),
        "# 日期\t汉字\t中文词\t英文词\t上屏次数\n2026-09-05\t12\t3\t1\t4\n坏行\n2026-09-05\t1\t1\t0\t1\n",
    )
    .unwrap();
    // 末尾是被打断的半行
    std::fs::write(
        dir.join(USAGE_FILE),
        "2026-09-06\t8\t5\t1\t0\t1\t1\t1\t0\t5\t4\t900\n2026-09-06\t9\t5\t1",
    )
    .unwrap();
    let mut stats = UsageStats::open(&dir);
    let days = stats.days();
    assert_eq!(days[&date("2026-09-05")].hanzi, 13);
    assert_eq!(days[&date("2026-09-06")].hanzi, 5);
    // 老记录不分时段
    assert_eq!(stats.hours_on(date("2026-09-05")), [Usage::default(); 24]);
    stats.record_at(at("2026-09-06T10:00"), usage(2));
    assert_eq!(UsageStats::open(&dir).days()[&date("2026-09-06")].hanzi, 7);
    // 老文件不动
    stats.save().unwrap();
    assert!(
        std::fs::read_to_string(dir.join("usage.tsv"))
            .unwrap()
            .contains("2026-09-05\t12")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn compacts_once_the_file_has_many_repeated_hours() {
    let dir = scratch("compact");
    let mut stats = UsageStats::open(&dir);
    for _ in 0..=COMPACT_AFTER {
        stats.record_at(at("2026-09-06T08:10"), usage(1));
    }
    stats.save().unwrap();
    let text = std::fs::read_to_string(dir.join(USAGE_FILE)).unwrap();
    assert_eq!(text.lines().count(), 2, "表头加一个时段");
    assert_eq!(
        UsageStats::open(&dir).days()[&date("2026-09-06")].hanzi,
        COMPACT_AFTER as u64 + 1
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
