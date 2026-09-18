//! 码表本体：词 → 码的映射、`.qj` 读写与查询。

use std::ops::Range;
use std::path::Path;

use qingjian_format::{Container, Kind, Metadata, Table, Text, Writer, hash};

use super::entry::CodeEntry;
use super::lookup::AuxCodeLookup;
use super::parsed::ParsedTable;
use crate::error::DictionaryError;

/// `.qj` 里的分节：词文本 arena、码键 arena、条目表、词 → 条目区间的哈希索引。
const TEXT_TAG: [u8; 4] = *b"TEXT";
const CODE_TAG: [u8; 4] = *b"CODE";
const ENTR_TAG: [u8; 4] = *b"ENTR";
const HASH_TAG: [u8; 4] = *b"HASH";

/// 码的最长字节数（`a-z`，1–8）。
pub const MAX_CODE_LEN: usize = 8;

/// 码是否合法：全是 `a-z`（大小写不敏感）、长度 1–`MAX_CODE_LEN`。
pub fn is_valid_code(code: &str) -> bool {
    !code.is_empty() && code.len() <= MAX_CODE_LEN && code.bytes().all(|b| b.is_ascii_lowercase())
}

/// 码表：词 → 码的映射。从 `.qj` 打开时四段数据都是映射文件里的一段，查询代码不区分。
#[derive(Debug, Default)]
pub struct AuxCodeTable {
    /// 所有词文本首尾相接。
    words: Text,

    /// 所有码首尾相接。
    codes: Text,

    /// 按词文本的字节序升序；同一个词的条目相邻。
    entries: Table<CodeEntry>,

    /// 词 → 条目区间起点的哈希索引（`hash::build` 的开放寻址表）。
    index: Table<u32>,

    /// 去重后的词数。
    word_count: usize,

    /// `.qj` 里的来历（名称、许可证、署名）；内存里造的没有。
    metadata: Option<Metadata>,
}

impl AuxCodeTable {
    /// 用 `(词, 码)` 对造一张表。
    ///
    /// 重复的 `(词, 码)` 去掉；同一个词的多条码都保留（一词多码）。码非法时报错，
    /// 调用方（导入）应当先过滤并统计，不要让坏数据走到这里。
    pub fn from_pairs(
        pairs: impl IntoIterator<Item = (String, String)>,
    ) -> Result<Self, DictionaryError> {
        let mut rows: Vec<(String, String)> = pairs.into_iter().collect();
        for (_, code) in &rows {
            if !is_valid_code(code) {
                return Err(DictionaryError::InvalidCode(code.clone()));
            }
        }
        rows.sort();
        rows.dedup();
        let mut words = String::new();
        let mut codes = String::new();
        let mut entries: Vec<CodeEntry> = Vec::with_capacity(rows.len());
        let mut previous: Option<&str> = None;
        let mut word_count = 0;
        for (word, code) in &rows {
            if previous != Some(word.as_str()) {
                word_count += 1;
                previous = Some(word.as_str());
            }
            let word_len = u16::try_from(word.len())
                .map_err(|_| DictionaryError::Corrupt("word too long for a code table"))?;
            entries.push(CodeEntry {
                word_start: words.len() as u32,
                code_start: codes.len() as u32,
                word_len,
                code_len: code.len() as u8,
                reserved: 0,
            });
            words.push_str(word);
            codes.push_str(code);
        }
        let index = hash::build(entries.len(), |id| {
            let entry = entries[id as usize];
            &words[entry.word_start as usize..entry.word_start as usize + entry.word_len as usize]
        });
        tracing::debug!(entries = entries.len(), words = word_count, "码表已建立");
        Ok(Self {
            words: Text::Owned(words),
            codes: Text::Owned(codes),
            entries: Table::Owned(entries),
            index: Table::Owned(index),
            word_count,
            metadata: None,
        })
    }

    /// 从 TSV 解析：每行 `词\t码`，`#` 开头为注释，空行忽略；同一个词可以有多行。
    pub fn parse(source: &str) -> Result<Self, DictionaryError> {
        let mut pairs = Vec::new();
        for (index, raw) in source.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let number = index + 1;
            let mut fields = line.split('\t');
            let word = fields
                .next()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or(DictionaryError::Line {
                    line: number,
                    reason: "missing word",
                })?;
            let code = fields
                .next()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or(DictionaryError::Line {
                    line: number,
                    reason: "missing code",
                })?;
            if !is_valid_code(code) {
                return Err(DictionaryError::InvalidCode(code.to_owned()));
            }
            pairs.push((word.to_owned(), code.to_owned()));
        }
        Self::from_pairs(pairs)
    }

    /// 按文件内容选加载方式：`.qj` 容器直接映射；文本再按内容分 Rime `.dict.yaml` 与 `词\t码` TSV。
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, DictionaryError> {
        let path = path.as_ref();
        if Container::is_qj(path) {
            Self::open(path)
        } else {
            Self::from_text(&std::fs::read_to_string(path)?)
        }
    }

    /// 解析码表文本：有 YAML 头（`---` 或 `name:` 打头）的按 Rime 规则读——与「导入码表」走同一套
    /// 解析（`columns` / `import_tables` / 「有词无码」统计口径一致），其余按 `词\t码` 的 TSV 读。
    pub fn from_text(text: &str) -> Result<Self, DictionaryError> {
        if !looks_like_rime(text) {
            return Self::parse(text);
        }
        let parsed = ParsedTable::parse(text);
        if parsed.pairs.is_empty() {
            return Err(DictionaryError::NoCodeEntries);
        }
        Self::from_pairs(parsed.pairs)
    }

    /// 打开 `.qj` 码表：映射四个分节，逐条校验偏移落在 arena 内、条目按词有序、哈希索引自洽。
    pub fn open(path: &Path) -> Result<Self, DictionaryError> {
        let container = Container::open(path, Kind::AuxCodeTable)?;
        let words = container.text(TEXT_TAG)?;
        let codes = container.text(CODE_TAG)?;
        let entries: Table<CodeEntry> = container.table(ENTR_TAG)?;
        let index: Table<u32> = container.table(HASH_TAG)?;
        if !hash::is_valid(&index, entries.len()) {
            return Err(DictionaryError::Corrupt(
                "code table hash index is malformed",
            ));
        }
        let mut word_count = 0;
        let mut previous: Option<&str> = None;
        for entry in entries.iter() {
            let start = entry.word_start as usize;
            let word = words
                .get(start..start + usize::from(entry.word_len))
                .ok_or(DictionaryError::Corrupt(
                    "code entry points outside the word arena",
                ))?;
            let start = entry.code_start as usize;
            let code = codes
                .get(start..start + usize::from(entry.code_len))
                .ok_or(DictionaryError::Corrupt(
                    "code entry points outside the code arena",
                ))?;
            if !is_valid_code(code) {
                return Err(DictionaryError::Corrupt("code is not a-z of length 1-8"));
            }
            if previous.is_none_or(|last| last < word) {
                word_count += 1;
            } else if previous != Some(word) {
                return Err(DictionaryError::Corrupt(
                    "code entries are not sorted by word",
                ));
            }
            previous = Some(word);
        }
        tracing::debug!(
            entries = entries.len(),
            words = word_count,
            name = %container.metadata().name,
            "码表已映射"
        );
        Ok(Self {
            words,
            codes,
            entries,
            index,
            word_count,
            metadata: Some(container.metadata().clone()),
        })
    }

    /// 写成 `.qj`：内存里的四段原样落盘。`metadata.entries` 会填成词条数。
    pub fn write_qj(&self, path: &Path, metadata: &Metadata) -> Result<(), DictionaryError> {
        let metadata = Metadata {
            entries: self.entries.len() as u64,
            ..metadata.clone()
        };
        Writer::new(Kind::AuxCodeTable, &metadata)?
            .section(TEXT_TAG, self.words.as_bytes())
            .section(CODE_TAG, self.codes.as_bytes())
            .section(ENTR_TAG, self.entries.as_bytes())
            .section(HASH_TAG, self.index.as_bytes())
            .write_to(path)?;
        Ok(())
    }

    /// `.qj` 里的来历；内存里造的返回 `None`。
    pub fn metadata(&self) -> Option<&Metadata> {
        self.metadata.as_ref()
    }

    /// 词条数（词 → 码 的条目数，一词多码算多条）。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 去重后的词数。
    pub fn word_count(&self) -> usize {
        self.word_count
    }

    /// `word` 的全部码，按文件里的顺序。没有这个词时是空迭代器。
    pub fn codes_of<'a>(&'a self, word: &str) -> impl Iterator<Item = &'a str> + 'a {
        let range = self.range_of(word);
        self.entries[range].iter().map(|entry| self.code(entry))
    }

    /// 全部条目，按词文本的字节序升序。
    pub fn entries(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
        self.entries
            .iter()
            .map(|entry| (self.word(entry), self.code(entry)))
    }

    /// 词 → 条目区间。没有这个词时是空区间。
    fn range_of(&self, word: &str) -> Range<usize> {
        let Some(first) = hash::find(&self.index, word, |id| {
            self.word(&self.entries[id as usize])
        }) else {
            return 0..0;
        };
        let start = first as usize;
        let length = self.entries[start..].partition_point(|entry| self.word(entry) == word);
        start..start + length
    }

    fn word(&self, entry: &CodeEntry) -> &str {
        let start = entry.word_start as usize;
        &self.words[start..start + usize::from(entry.word_len)]
    }

    fn code(&self, entry: &CodeEntry) -> &str {
        let start = entry.code_start as usize;
        &self.codes[start..start + usize::from(entry.code_len)]
    }
}

/// 文本像不像 Rime 词库 / 码表：第一条非注释非空行是 `---` 或以 `name:` 打头。
fn looks_like_rime(text: &str) -> bool {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .is_some_and(|line| line == "---" || line.starts_with("name:"))
}

impl AuxCodeLookup for AuxCodeTable {
    fn code_with_prefix<'a>(&'a self, word: &str, prefix: &str) -> Option<&'a str> {
        // inherent 的 codes_of 是迭代器版本，这里要用它（别递归回 trait）
        AuxCodeTable::codes_of(self, word).find(|code| code.starts_with(prefix))
    }

    fn codes_of<'a>(&'a self, word: &str) -> Vec<&'a str> {
        AuxCodeTable::codes_of(self, word).collect()
    }
}
