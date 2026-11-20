//! 形码码表：按编码查字词（五笔）。
//!
//! 与 [`crate::Dictionary`] 的区别在键：词库按拼音音节序列查，码表按编码本身查，
//! 敲的编码是词的编码的前缀即可命中（`gan` 命中编码 `gant` 的 开发），没有音节也没有切分。
//!
//! 文件格式 TSV：
//!
//! ```text
//! 词\t编码\t词频
//! 开发\tgant\t9000
//! ```
//!
//! `#` 开头为注释行，空行忽略。编码统一按小写存，查询输入也须已小写；编码里有字母以外的字符的行跳过。
//!
//! 随包方案的码表打成 `.qj`（`Kind::CodeTable`，`dict-convert pack codetable`）：方案的名字、作者、许可、版本、简介写在元数据里，
//! `TEXT` 一节是按原行序写回的 TSV，打开时照旧解析。

mod index;
mod parse;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

/// `.qj` 里放 TSV 原文的那一节。
const TEXT_TAG: [u8; 4] = *b"TEXT";

/// 一条码表词目。
#[derive(Debug, Clone)]
pub(super) struct Entry {
    /// 全码，小写。
    code: String,

    /// 码表中的行序；单码精确命中按此顺序选候选。
    order: usize,

    /// 词。
    text: String,

    /// 静态词频。
    frequency: u32,
}

/// 形码码表。词目按 `(编码, 词频降序)` 排好，同前缀的是一段连续区间。
///
/// `Clone` 是给回放用的：`Engine::set_code_table` 收所有权，而回放要在方案之间来回切，
/// 手里得留一份（整份日志通常只有一套方案，切的次数很少）。
#[derive(Debug, Default, Clone)]
pub struct CodeTable {
    /// 按编码字节序升序，同一个编码下按词频降序。
    entries: Vec<Entry>,

    /// 全部词频之和，上下文得分的兜底用。
    total_frequency: u64,

    /// 满码有几个字母（五笔是 4）：码表里最常见的编码长度。全码占绝大多数，取最常见的而不是最长的，
    /// 个别写错的长编码（极点码表里 `TF卡` 是 `ttfhh`）不至于把满码拖成 5。
    full_code_len: usize,

    /// 新合并码表的行序起点。
    next_order: usize,
    full_codes: BTreeMap<char, String>,
}

impl CodeTable {
    /// 单字最长编码；同长时按码表排序取第一条，不用用户敲入的简码取词码。
    pub fn full_code(&self, character: char) -> Option<&str> {
        self.full_codes.get(&character).map(String::as_str)
    }

    /// 全部词频之和。
    pub fn total_frequency(&self) -> u64 {
        self.total_frequency
    }

    /// 满码的字母数（最常见的编码长度）；空表为 0。
    pub fn full_code_len(&self) -> usize {
        self.full_code_len
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 测试用：全部词目 `(词, 编码, 词频)`，按码表内部顺序。
    #[cfg(test)]
    fn entries_view(&self) -> impl Iterator<Item = (&str, &str, u32)> + '_ {
        self.entries
            .iter()
            .map(|entry| (entry.text.as_str(), entry.code.as_str(), entry.frequency))
    }
}
