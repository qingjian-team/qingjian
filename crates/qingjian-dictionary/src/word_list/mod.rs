//! 词表（英文、越南语……）：按小写编码查原样大小写的词，也能按前缀补全。
//!
//! 9.5 万词全常驻，所以不给每个词单独分配：编码与原样词拼进一块文字缓冲，条目只记位置（[`Entry`]），按编码排序后二分查。
//! 打包成 `.qj` 时这两块原样落盘，打开时直接映射，不用再解析、排序。

mod entry;

use std::collections::HashSet;
use std::path::Path;

use qingjian_format::{Container, Kind, Metadata, Table, Text, Writer};

use crate::error::DictionaryError;
use entry::Entry;

/// `.qj` 里的分节：文字缓冲、按编码排好的条目表。
const TEXT_TAG: [u8; 4] = *b"TEXT";
const ENTRIES_TAG: [u8; 4] = *b"ENTR";

/// 文件格式 TSV：`词\t编码[\t词频]`（编码缺省为词的小写，词频缺省 0），或 `pack wordlist` 打的 `.qj`。
#[derive(Debug, Default)]
pub struct WordList {
    /// 所有编码与原样词拼在一起。
    text: Text,

    /// 按编码字节序排好：精确查与前缀补全都二分定位。
    entries: Table<Entry>,
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
        tracing::debug!(words = entries.len(), "词表加载完成");
        Ok(Self {
            text: Text::Owned(text),
            entries: Table::Owned(entries),
        })
    }

    /// 按文件内容选加载方式：`.qj` 直接映射，否则当 TSV 解析。
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, DictionaryError> {
        let path = path.as_ref();
        if Container::is_qj(path) {
            Self::open_qj(path)
        } else {
            Self::parse(&std::fs::read_to_string(path)?)
        }
    }

    /// 打开 `.qj` 词表：逐条校验偏移落在文字缓冲内、在字符边界上，条目按编码严格递增（二分查靠它）。
    pub fn open_qj(path: &Path) -> Result<Self, DictionaryError> {
        let container = Container::open(path, Kind::WordList)?;
        let text = container.text(TEXT_TAG)?;
        let entries: Table<Entry> = container.table(ENTRIES_TAG)?;
        let slice =
            |start: u32, len: u16| text.get(start as usize..start as usize + usize::from(len));
        let mut previous: Option<&str> = None;
        for entry in entries.iter() {
            let code = slice(entry.code_start, entry.code_len)
                .filter(|code| !code.is_empty())
                .ok_or(DictionaryError::Corrupt(
                    "word list code points outside the text",
                ))?;
            if slice(entry.word_start, entry.word_len).is_none_or(str::is_empty) {
                return Err(DictionaryError::Corrupt(
                    "word list word points outside the text",
                ));
            }
            if previous.is_some_and(|last| last >= code) {
                return Err(DictionaryError::Corrupt(
                    "word list entries are not sorted by code",
                ));
            }
            previous = Some(code);
        }
        tracing::debug!(
            words = entries.len(),
            name = %container.metadata().name,
            "词表已映射"
        );
        Ok(Self { text, entries })
    }

    /// 写成 `.qj`：文字缓冲与条目表原样落盘。`metadata.entries` 会填成词数。
    pub fn write_qj(&self, path: &Path, metadata: &Metadata) -> Result<(), DictionaryError> {
        let metadata = Metadata {
            entries: self.entries.len() as u64,
            ..metadata.clone()
        };
        Writer::new(Kind::WordList, &metadata)?
            .section(TEXT_TAG, self.text.as_bytes())
            .section(ENTRIES_TAG, self.entries.as_bytes())
            .write_to(path)?;
        Ok(())
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

    #[test]
    fn qj_round_trip_keeps_lookups() {
        let dir = std::env::temp_dir().join(format!("qingjian-word-list-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("words.qj");
        let source = "GitHub\tgithub\t700\ncompany\tcompany\t900\ncompare\tcompare\t500\ntiếng Việt\ttiếng việt\t20\n";
        WordList::parse(source)
            .unwrap()
            .write_qj(&path, &Metadata::default())
            .unwrap();
        let list = WordList::from_path(&path).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(list.len(), 4);
        assert_eq!(list.get("github"), Some("GitHub"));
        assert_eq!(list.frequency("company"), Some(900));
        assert_eq!(list.complete("comp", 5), ["company", "compare"]);
        assert_eq!(list.complete("tiếng", 5), ["tiếng Việt"]);
    }
}
