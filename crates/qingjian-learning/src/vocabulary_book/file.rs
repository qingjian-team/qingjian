//! `user-vocab.tsv` 的读写：一个译词一行
//! `语言\t译词\t看到轮次\t上屏次数\t用过次数\t首见\t末见\t认识\t中文\t例句`。
//! 后三列是后来加的：只有前七列的老文件照读，老版本读新文件也只取前七列。

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;
use std::time::SystemTime;

use super::VocabularyEntry;

/// (语言代码, 译词)。
pub(super) type Key = (String, String);

pub(super) const HEADER: &str =
    "# 语言\t译词\t看到轮次\t上屏次数\t用过次数\t首见\t末见\t认识\t中文\t例句\n";

/// 文件的修改时间与长度：两边（输入法、设置程序）各自写，靠它看出文件被对方改过。
pub(super) fn stamp(path: &Path) -> Option<(SystemTime, u64)> {
    let metadata = std::fs::metadata(path).ok()?;
    Some((metadata.modified().ok()?, metadata.len()))
}

/// 按行解析；格式不对的行跳过（记警告），重复的键取后一条。
pub(super) fn parse(text: &str) -> BTreeMap<Key, VocabularyEntry> {
    let mut entries = BTreeMap::new();
    for (number, line) in text.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match parse_line(line) {
            Some((key, entry)) => {
                entries.insert(key, entry);
            }
            None => tracing::warn!(line = number + 1, "词汇记录有坏行，跳过"),
        }
    }
    entries
}

fn parse_line(line: &str) -> Option<(Key, VocabularyEntry)> {
    let mut fields = line.split('\t');
    let code = fields.next()?.to_owned();
    let word = fields.next()?.to_owned();
    if code.is_empty() || word.is_empty() {
        return None;
    }
    let mut number = || fields.next()?.parse::<u32>().ok();
    let seen = number()?;
    let committed = number()?;
    let used = number()?;
    let first = fields.next()?.parse().ok()?;
    let last = fields.next()?.parse().ok()?;
    let known = fields.next() == Some("1");
    let source = fields.next().unwrap_or_default().to_owned();
    let example = fields.next().unwrap_or_default().to_owned();
    Some((
        (code, word),
        VocabularyEntry {
            seen,
            committed,
            used,
            first,
            last,
            known,
            source,
            example,
        },
    ))
}

pub(super) fn render(entries: &BTreeMap<Key, VocabularyEntry>) -> String {
    let mut text = String::from(HEADER);
    for ((code, word), entry) in entries {
        let _ = writeln!(
            text,
            "{code}\t{word}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            entry.seen,
            entry.committed,
            entry.used,
            entry.first,
            entry.last,
            u8::from(entry.known),
            entry.source,
            entry.example,
        );
    }
    text
}

/// 例句与中文进文件前去掉会破坏行格式的字符。
pub(super) fn one_line(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{HEADER, one_line, parse, render, stamp};
    use crate::vocabulary_book::VocabularyEntry;
    use std::collections::BTreeMap;

    fn date(text: &str) -> jiff::civil::Date {
        text.parse().unwrap()
    }

    #[test]
    fn the_old_seven_column_file_still_reads() {
        let text = format!("{HEADER}ja\tこんにちは\t3\t1\t1\t2026-11-01\t2026-11-20\n");
        let entries = parse(&text);
        let entry = &entries[&("ja".to_owned(), "こんにちは".to_owned())];
        assert_eq!((entry.seen, entry.committed, entry.used), (3, 1, 1));
        assert!(!entry.known);
        assert_eq!((entry.source.as_str(), entry.example.as_str()), ("", ""));
        assert_eq!(
            (entry.first, entry.last),
            (date("2026-11-01"), date("2026-11-20"))
        );
    }

    #[test]
    fn render_round_trips_the_last_three_columns() {
        let mut entries = BTreeMap::new();
        let mut entry = VocabularyEntry::new(date("2026-11-01"));
        entry.seen = 5;
        entry.committed = 2;
        entry.used = 1;
        entry.last = date("2026-11-20");
        entry.known = true;
        entry.source = "你好".into();
        entry.example = "你好，世界".into();
        entries.insert(("ja".to_owned(), "こんにちは".to_owned()), entry);
        let text = render(&entries);
        assert!(text.starts_with(HEADER));
        let parsed = parse(&text);
        let got = &parsed[&("ja".to_owned(), "こんにちは".to_owned())];
        assert_eq!((got.seen, got.committed, got.used), (5, 2, 1));
        assert!(got.known);
        assert_eq!(
            (got.source.as_str(), got.example.as_str()),
            ("你好", "你好，世界")
        );
        assert_eq!(
            (got.first, got.last),
            (date("2026-11-01"), date("2026-11-20"))
        );
    }

    #[test]
    fn bad_lines_are_skipped_and_later_duplicates_win() {
        let text = format!(
            "{HEADER}ja\tこんにちは\t1\t0\t0\t2026-11-01\t2026-11-01\n\
             ja\tこんにちは\t9\t0\t0\t2026-11-01\t2026-11-01\n\
             zh\t\n\
             x\tx\t1\t0\t0\t坏日期\t2026-11-01\n"
        );
        let entries = parse(&text);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[&("ja".to_owned(), "こんにちは".to_owned())].seen, 9);
    }

    #[test]
    fn one_line_blanks_control_characters_and_trims() {
        assert_eq!(one_line(" 你好\t世界\n"), "你好 世界");
        assert_eq!(one_line("a\u{0}b"), "a b");
        assert_eq!(one_line(""), "");
    }

    #[test]
    fn stamp_of_a_missing_file_is_none() {
        let path = std::env::temp_dir().join("qj-vocab-file-never.tsv");
        let _ = std::fs::remove_file(&path);
        assert!(stamp(&path).is_none());
    }
}
