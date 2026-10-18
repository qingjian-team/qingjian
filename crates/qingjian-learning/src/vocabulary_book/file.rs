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
