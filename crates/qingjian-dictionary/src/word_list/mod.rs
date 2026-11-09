//! 英文词表：按小写编码查原样大小写的词，也能按前缀补全。
//!
//! 9.5 万词全常驻，所以不给每个词单独分配：编码与原样词拼进一块文字缓冲，条目只记位置（[`Entry`]），按编码排序后二分查。

mod entry;

use std::collections::HashSet;
use std::path::Path;

use crate::error::DictionaryError;
use entry::Entry;

/// 文件格式 TSV：`词\t编码[\t词频]`（编码缺省为词的小写，词频缺省 0）。
#[derive(Debug, Default)]
pub struct WordList {
    /// 所有编码与原样词拼在一起。
    text: String,

    /// 按编码字节序排好：精确查与前缀补全都二分定位。
    entries: Vec<Entry>,
}

impl WordList {
    pub fn parse(source: &str) -> Result<Self, DictionaryError> {
        let mut seen: HashSet<String> = HashSet::new();
        let mut rows: Vec<(String, &str, u32)> = Vec::new();
        for (index, raw) in source.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            let word = fields
                .next()
                .filter(|s| !s.is_empty())
                .ok_or(DictionaryError::Line {
                    line: index + 1,
                    reason: "missing word",
                })?;
            let code = fields
                .next()
                .filter(|s| !s.is_empty())
                .map(str::to_ascii_lowercase)
                .unwrap_or_else(|| word.to_ascii_lowercase());
            let frequency = fields
                .next()
                .map(|s| s.trim().parse::<u32>())
                .transpose()
                .map_err(|_| DictionaryError::Line {
                    line: index + 1,
                    reason: "frequency is not a non-negative integer",
                })?
                .unwrap_or(0);
            // 同一编码取第一个（词表按常用度排序时即最常用的写法）
            if seen.insert(code.clone()) {
                rows.push((code, word, frequency));
            }
        }
        drop(seen);
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        let mut text = String::with_capacity(rows.iter().map(|(code, _, _)| code.len()).sum());
        let mut entries = Vec::with_capacity(rows.len());
        for (code, word, frequency) in rows {
            let too_long = || DictionaryError::Line {
                line: 0,
                reason: "word longer than 65535 bytes",
            };
            let code_start = text.len() as u32;
            text.push_str(&code);
            let word_start = if word == code {
                code_start
            } else {
                let start = text.len() as u32;
                text.push_str(word);
                start
            };
            entries.push(Entry {
                code_start,
                code_len: u16::try_from(code.len()).map_err(|_| too_long())?,
                word_start,
                word_len: u16::try_from(word.len()).map_err(|_| too_long())?,
                frequency,
            });
        }
        text.shrink_to_fit();
        tracing::debug!(words = entries.len(), "英文词表加载完成");
        Ok(Self { text, entries })
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, DictionaryError> {
        Self::parse(&std::fs::read_to_string(path)?)
    }

    fn code(&self, entry: &Entry) -> &str {
        let start = entry.code_start as usize;
        &self.text[start..start + usize::from(entry.code_len)]
    }

    fn word(&self, entry: &Entry) -> &str {
        let start = entry.word_start as usize;
        &self.text[start..start + usize::from(entry.word_len)]
    }

    fn find(&self, code: &str) -> Option<&Entry> {
        self.entries
            .binary_search_by(|entry| self.code(entry).cmp(code))
            .ok()
            .map(|index| &self.entries[index])
    }

    /// 输入（已小写）对应的英文词。
    pub fn get(&self, code: &str) -> Option<&str> {
        self.find(code).map(|entry| self.word(entry))
    }

    /// 输入（已小写）对应的词频；词表里没有为 `None`。
    pub fn frequency(&self, code: &str) -> Option<u32> {
        self.find(code).map(|entry| entry.frequency)
    }

    /// 以 `prefix` 开头（不含正好相等的）的词里词频最高的 `limit` 个，按词频降序。
    pub fn complete(&self, prefix: &str, limit: usize) -> Vec<&str> {
        if prefix.is_empty() || limit == 0 {
            return Vec::new();
        }
        let start = self
            .entries
            .partition_point(|entry| self.code(entry) < prefix);
        let mut hits: Vec<&Entry> = self.entries[start..]
            .iter()
            .take_while(|entry| self.code(entry).starts_with(prefix))
            .filter(|entry| self.code(entry) != prefix)
            .collect();
        hits.sort_by(|a, b| {
            b.frequency
                .cmp(&a.frequency)
                .then_with(|| self.code(a).cmp(self.code(b)))
        });
        hits.into_iter()
            .take(limit)
            .map(|entry| self.word(entry))
            .collect()
    }

    /// 全部词目：(小写编码, 原样词, 词频)，按编码字节序。
    pub fn entries(&self) -> impl Iterator<Item = (&str, &str, u32)> {
        self.entries
            .iter()
            .map(|entry| (self.code(entry), self.word(entry), entry.frequency))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_up_by_lowercase_code() {
        let list = WordList::parse("GitHub\tgithub\nhello\thello\niPhone\n").unwrap();
        assert_eq!(list.get("github"), Some("GitHub"));
        assert_eq!(list.get("iphone"), Some("iPhone"));
        assert_eq!(list.get("hello"), Some("hello"));
        assert_eq!(list.get("nope"), None);
    }

    #[test]
    fn completes_prefixes_by_frequency() {
        let list = WordList::parse(
            "compass\tcompass\t300\ncompany\tcompany\t900\ncompare\tcompare\t500\ncom\tcom\t100\ncomma\tcomma\n",
        )
        .unwrap();
        assert_eq!(list.complete("comp", 2), ["company", "compare"]);
        assert_eq!(
            list.complete("com", 10),
            ["company", "compare", "compass", "comma"]
        );
        assert!(list.complete("zzz", 3).is_empty());
        assert!(list.complete("", 3).is_empty());
    }
}
