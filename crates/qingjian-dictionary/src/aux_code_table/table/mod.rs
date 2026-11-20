//! 码表本体：词 → 码的映射、`.qj` 读写与查询。

mod load;
mod query;

use std::ops::Range;

use qingjian_format::{Metadata, Table, Text, hash};

use super::entry::CodeEntry;
use super::word_rules::WordRules;

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

    /// 表里没有的词按它从单字的码推导；不落盘，由方案的 `scheme.toml` 给。
    word_rules: WordRules,
}

impl AuxCodeTable {
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
