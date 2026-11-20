//! 词表 / 音标表的解析、装配、打开与落盘。

use std::collections::HashSet;
use std::path::Path;

use qingjian_format::{Container, Kind, Metadata, Table, Text, Writer};

use crate::error::DictionaryError;

use super::{ENTRIES_TAG, Entry, TEXT_TAG, WordList};

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
        Self::from_rows(rows, "word longer than 65535 bytes", "english words")
    }

    /// 词表与音标表共用的装配：按编码排序、去重、拼进一块文字缓冲。
    ///
    /// `word` 与 `code` 相同时只存一份（省掉重复的 `hello\thello`），
    /// 这正是两个表都能共用的原因——结构一样，只是「第二列」的含义不同。
    fn from_rows(
        mut rows: Vec<(String, &str, u32)>,
        too_long: &'static str,
        kind: &'static str,
    ) -> Result<Self, DictionaryError> {
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        rows.dedup_by(|a, b| a.0 == b.0);

        let mut text = String::with_capacity(rows.iter().map(|(code, _, _)| code.len()).sum());
        let mut entries = Vec::with_capacity(rows.len());
        for (code, word, frequency) in rows {
            let too_long = || DictionaryError::Line {
                line: 0,
                reason: too_long,
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
        tracing::debug!(words = entries.len(), kind, "word list parsed");
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

    /// 音标表的 [`Self::from_path`]：`.qj`（`pack pronunciations` 打的）直接映射，否则按 [`Self::parse_pronunciations`] 解析。
    pub fn pronunciations_from_path(path: impl AsRef<Path>) -> Result<Self, DictionaryError> {
        let path = path.as_ref();
        if Container::is_qj(path) {
            Self::open_qj(path)
        } else {
            Self::parse_pronunciations(&std::fs::read_to_string(path)?)
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

    /// 解析音标表（`词\t/音标/`）：第一列是词（转小写当编码），第二列是音标。
    ///
    /// 不能直接用 [`Self::parse`]——那个把第二列当编码（词表是 `词\t编码`），
    /// 在这里会把音标当成编码存进去，查什么都查不到。
    pub fn parse_pronunciations(source: &str) -> Result<Self, DictionaryError> {
        let mut rows: Vec<(String, &str, u32)> = Vec::new();
        for (index, raw) in source.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t').map(str::trim);
            let word = fields.next().filter(|s| !s.is_empty());
            let ipa = fields.next().filter(|s| !s.is_empty());
            let (Some(word), Some(ipa)) = (word, ipa) else {
                return Err(DictionaryError::Line {
                    line: index + 1,
                    reason: "expected `word\\tpronunciation`",
                });
            };
            rows.push((word.to_ascii_lowercase(), ipa, 0));
        }
        Self::from_rows(
            rows,
            "pronunciation longer than 65535 bytes",
            "pronunciations",
        )
    }
}
