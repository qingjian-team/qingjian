//! 码表的解析、装配、合并、落盘与自造词插入。

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use qingjian_format::{Container, Kind, Metadata, Writer};

use crate::error::DictionaryError;

use super::{CodeTable, Entry, TEXT_TAG};

impl CodeTable {
    pub fn parse(source: &str) -> Result<Self, DictionaryError> {
        let mut entries: Vec<Entry> = Vec::new();
        let mut untypable = 0usize;
        for (index, raw) in source.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let number = index + 1;
            let mut fields = line.split('\t');
            let text = fields
                .next()
                .filter(|s| !s.is_empty())
                .ok_or(DictionaryError::Line {
                    line: number,
                    reason: "missing word",
                })?;
            let code = fields
                .next()
                .filter(|s| !s.is_empty())
                .ok_or(DictionaryError::Line {
                    line: number,
                    reason: "missing code",
                })?;
            let frequency = fields
                .next()
                .map(|s| s.trim().parse::<u32>())
                .transpose()
                .map_err(|_| DictionaryError::Line {
                    line: number,
                    reason: "frequency is not a non-negative integer",
                })?
                .unwrap_or(1);
            // 编码只能是字母：别的键（`;` `,`）进不了形码查询，这种行收了也打不出来
            if !code.chars().all(|c| c.is_ascii_alphabetic()) {
                untypable += 1;
                continue;
            }
            entries.push(Entry {
                code: code.to_ascii_lowercase(),
                order: index,
                text: text.to_owned(),
                frequency,
            });
        }
        sort_and_dedup(&mut entries);
        let total_frequency = entries.iter().map(|e| u64::from(e.frequency)).sum();
        let full_code_len = most_common_len(&entries);
        let next_order = entries
            .iter()
            .map(|entry| entry.order)
            .max()
            .map_or(0, |order| order + 1);
        if untypable > 0 {
            tracing::warn!(untypable, "code table rows with non-letter codes skipped");
        }
        tracing::debug!(entries = entries.len(), full_code_len, "code table loaded");
        Ok(Self {
            full_codes: full_codes(&entries),
            entries,
            total_frequency,
            full_code_len,
            next_order,
        })
    }

    /// 按文件内容选加载方式：`.qj` 取出 `TEXT` 一节解析，否则当 TSV 读。
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, DictionaryError> {
        let path = path.as_ref();
        if Container::is_qj(path) {
            Self::parse(&Container::open(path, Kind::CodeTable)?.text(TEXT_TAG)?)
        } else {
            Self::parse(&std::fs::read_to_string(path)?)
        }
    }

    /// 写成 `.qj`：词目按码表原来的行序写回 TSV（单码按行序出候选，顺序不能丢）。`metadata.entries` 会填成词目数。
    pub fn write_qj(&self, path: &Path, metadata: &Metadata) -> Result<(), DictionaryError> {
        let mut rows: Vec<&Entry> = self.entries.iter().collect();
        rows.sort_by_key(|entry| entry.order);
        let mut text = String::new();
        for entry in rows {
            let _ = writeln!(text, "{}\t{}\t{}", entry.text, entry.code, entry.frequency);
        }
        let metadata = Metadata {
            entries: self.entries.len() as u64,
            ..metadata.clone()
        };
        Writer::new(Kind::CodeTable, &metadata)?
            .section(TEXT_TAG, text.as_bytes())
            .write_to(path)?;
        Ok(())
    }

    /// 把用户的形码词库并进来。满码长度仍按原表算：几条自造词不该改掉方案的码长。
    pub fn merge(&mut self, other: CodeTable) {
        if self.entries.is_empty() {
            self.full_code_len = other.full_code_len;
        }
        let order_offset = self.next_order;
        self.entries
            .extend(other.entries.into_iter().map(|mut entry| {
                entry.order += order_offset;
                entry
            }));
        self.next_order += other.next_order;
        sort_and_dedup(&mut self.entries);
        self.total_frequency = self.entries.iter().map(|e| u64::from(e.frequency)).sum();
        self.full_codes = full_codes(&self.entries);
    }

    /// 插入新组词，不重排整张表或改变单字简码顺序；相同词码已存在时不重复插入。
    pub fn insert_phrase(&mut self, text: &str, code: &str, frequency: u32) -> bool {
        if !(2..=10).contains(&text.chars().count())
            || code.is_empty()
            || !code.chars().all(|character| character.is_ascii_lowercase())
        {
            return false;
        }
        let start = self
            .entries
            .partition_point(|entry| entry.code.as_str() < code);
        let end = self
            .entries
            .partition_point(|entry| entry.code.as_str() <= code);
        if self.entries[start..end]
            .iter()
            .any(|entry| entry.text == text)
        {
            return false;
        }
        let offset = self.entries[start..end].partition_point(|entry| {
            entry.frequency > frequency
                || (entry.frequency == frequency && entry.text.as_str() < text)
        });
        self.entries.insert(
            start + offset,
            Entry {
                code: code.to_owned(),
                order: self.next_order,
                text: text.to_owned(),
                frequency,
            },
        );
        self.next_order += 1;
        self.total_frequency += u64::from(frequency);
        true
    }
}

fn full_codes(entries: &[Entry]) -> BTreeMap<char, String> {
    let mut codes: BTreeMap<char, String> = BTreeMap::new();
    for entry in entries {
        let mut chars = entry.text.chars();
        if let Some(character) = chars.next()
            && chars.next().is_none()
            && codes
                .get(&character)
                .is_none_or(|code| code.len() < entry.code.len())
        {
            codes.insert(character, entry.code.clone());
        }
    }
    codes
}

/// 按 `(编码, 词频降序)` 排好；同一个词记了两遍（不同码表版本合并、自造词与原表重了）留词频高的那条，排完序就是靠前的那条。
fn sort_and_dedup(entries: &mut Vec<Entry>) {
    entries.sort_by(|a, b| {
        a.code
            .cmp(&b.code)
            .then_with(|| b.frequency.cmp(&a.frequency))
            .then_with(|| a.text.cmp(&b.text))
    });
    entries.dedup_by(|a, b| a.code == b.code && a.text == b.text);
}

/// 最常见的编码长度，一样多时取长的。
fn most_common_len(entries: &[Entry]) -> usize {
    let mut counts: Vec<usize> = Vec::new();
    for entry in entries {
        let len = entry.code.len();
        if counts.len() <= len {
            counts.resize(len + 1, 0);
        }
        counts[len] += 1;
    }
    counts
        .iter()
        .enumerate()
        .max_by_key(|&(len, count)| (*count, len))
        .map_or(0, |(len, _)| len)
}
