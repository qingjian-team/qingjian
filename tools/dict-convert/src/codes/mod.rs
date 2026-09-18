//! 笔画码表：笔画表（`stroke` 子命令的产物）+ 词库 → `codes/stroke.qj`（随包原生码表）。
//!
//! 取码规则照 `docs/design/aux-code.md` 的「原生表：只带笔画」：
//!
//! - 键位：CNS 的 1 → `h` 横、2 → `s` 竖、3 → `p` 撇、5 → `z` 折，点与捺（4 / `n`）→ `n`；
//! - 单字：前 4 笔 + 末笔（不足 5 笔按实际取，1 画的字就是 1 码）；
//! - 词组：每字首笔——二字 2 码、三字 3 码、四字及以上 = 前三字首笔 + 末字首笔（4 码）；
//! - 词里有一个字不在笔画表里，整词没有码、跳过并计入统计：宁可不出，也不用缺字的词凑半条码
//!   （四字以上按规则用不到中间的字，同样要求整词都在表里）。
//!
//! 产物是 `AuxCodeTable` 的 `.qj`（辅码态两段式查询用的 `Kind::AuxCodeTable` 容器）；元数据的缺省值
//! （名称「笔画」、OFL-1.1、CNS11643 署名）在 `pack` 里，许可依据见设计文档「许可」一节。

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Instant;

use qingjian_dictionary::{AuxCodeTable, Dictionary};
use qingjian_format::Metadata;

use crate::error::ConvertError;
use crate::stroke::{malformed, single_char};

#[cfg(test)]
mod tests;

/// 笔画表：字 → 键序列（`h` 横 / `s` 竖 / `p` 撇 / `n` 点捺 / `z` 折）。
pub struct StrokeTable {
    /// 字 → 键序列，序列非空。
    sequences: HashMap<char, String>,
}

impl StrokeTable {
    /// 读 `stroke` 子命令的产物：每行 `字\t序列`，序列是 `[1235n]+`，`#` 开头为注释。
    pub fn from_path(path: &Path) -> Result<Self, ConvertError> {
        Self::parse(path, BufReader::new(File::open(path)?))
    }

    /// 从任意读入流解析（测试用）。
    fn parse(path: &Path, reader: impl BufRead) -> Result<Self, ConvertError> {
        let mut sequences: HashMap<char, String> = HashMap::new();
        for (index, line) in reader.lines().enumerate() {
            let line = line?;
            let number = index + 1;
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            let (Some(ch), Some(sequence)) = (fields.next(), fields.next()) else {
                return Err(malformed(
                    path,
                    number,
                    "expected a character and a sequence",
                ));
            };
            let Some(ch) = single_char(ch) else {
                return Err(malformed(path, number, "expected one character"));
            };
            let Some(sequence) = key_sequence(sequence) else {
                return Err(malformed(
                    path,
                    number,
                    "expected a stroke sequence of 1 2 3 4 5 n",
                ));
            };
            if sequences.insert(ch, sequence).is_some() {
                return Err(malformed(
                    path,
                    number,
                    "two stroke sequences for one character",
                ));
            }
        }
        Ok(Self { sequences })
    }

    /// 一个字的键序列。
    pub fn get(&self, ch: char) -> Option<&str> {
        self.sequences.get(&ch).map(String::as_str)
    }

    /// 表里的字数。
    pub fn len(&self) -> usize {
        self.sequences.len()
    }

    /// 表是不是空的（空表时任何词都没有码）。
    pub fn is_empty(&self) -> bool {
        self.sequences.is_empty()
    }
}

/// 一次生成的统计。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CodeStats {
    /// 词库里的词条数（一个词多条读音算多条）。
    pub entries: usize,

    /// 去重后的词数。
    pub words: usize,

    /// 有码的词数。
    pub coded: usize,

    /// 词里有字不在笔画表里、跳过的词数。
    pub skipped: usize,
}

/// 生成码表：读笔画表与词库，按取码规则算码，写 `.qj`（词里缺字的词跳过）。
pub fn build(
    stroke: &Path,
    dict: &Path,
    out: &Path,
    metadata: &Metadata,
) -> Result<CodeStats, ConvertError> {
    let started = Instant::now();
    let strokes = StrokeTable::from_path(stroke)?;
    tracing::info!(path = %stroke.display(), chars = strokes.len(), "已读笔画表");
    if strokes.is_empty() {
        tracing::warn!(path = %stroke.display(), "笔画表是空的：不会有任何词有码");
    }
    let dictionary = Dictionary::from_path(dict)?;
    let (pairs, stats) = collect(&dictionary, &strokes);
    if stats.words == 0 {
        tracing::warn!(path = %dict.display(), "词库是空的：码表不会有条目");
    }
    let table = AuxCodeTable::from_pairs(pairs)?;
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    table.write_qj(out, metadata)?;

    let size = std::fs::metadata(out).map(|meta| meta.len()).unwrap_or(0);
    tracing::info!(
        out = %out.display(),
        dict = %dict.display(),
        dict_entries = stats.entries,
        words = stats.words,
        coded = stats.coded,
        skipped_no_code = stats.skipped,
        entries = table.len(),
        table_words = table.word_count(),
        size_kb = size / 1_000,
        elapsed_ms = started.elapsed().as_millis(),
        "已生成笔画码表"
    );
    Ok(stats)
}

/// 词库 → `(词, 码)` 对与统计；同一个词的重复词条（多音字）只算一次。
fn collect(dictionary: &Dictionary, strokes: &StrokeTable) -> (Vec<(String, String)>, CodeStats) {
    let mut stats = CodeStats {
        entries: dictionary.len(),
        ..CodeStats::default()
    };
    let mut seen: HashSet<&str> = HashSet::new();
    let mut pairs = Vec::new();
    for entry in dictionary.entries() {
        if !seen.insert(entry.text) {
            continue;
        }
        stats.words += 1;
        match word_code(entry.text, strokes) {
            Some(code) => {
                stats.coded += 1;
                pairs.push((entry.text.to_owned(), code));
            }
            None => stats.skipped += 1,
        }
    }
    (pairs, stats)
}

/// 一个词的码：单字走单字规则，词组每字首笔；词里有字不在表里就没有码。
fn word_code(word: &str, strokes: &StrokeTable) -> Option<String> {
    let mut sequences = Vec::new();
    for ch in word.chars() {
        sequences.push(strokes.get(ch)?);
    }
    match sequences.as_slice() {
        [] => None,
        [only] => single_char_code(only),
        [a, b] => heads(&[a, b]),
        [a, b, c] => heads(&[a, b, c]),
        many => heads(&[many[0], many[1], many[2], many[many.len() - 1]]),
    }
}

/// 单字码：前 4 笔 + 末笔；不足 5 笔按实际取。
fn single_char_code(sequence: &str) -> Option<String> {
    let mut keys: Vec<char> = sequence.chars().collect();
    match keys.len() {
        0 => None,
        1..=5 => Some(keys.into_iter().collect()),
        _ => {
            let last = keys.pop()?;
            let mut code: String = keys[..4].iter().collect();
            code.push(last);
            Some(code)
        }
    }
}

/// 词组码：每个给定字各取首笔。
fn heads(sequences: &[&str]) -> Option<String> {
    sequences
        .iter()
        .map(|sequence| sequence.chars().next())
        .collect()
}

/// 把一个字的 CNS 笔画序列映射成键；认不出的符号返回 `None`。
fn key_sequence(sequence: &str) -> Option<String> {
    if sequence.is_empty() {
        return None;
    }
    sequence.chars().map(key).collect()
}

/// 单个笔画码的键位（`stroke` 的产物已经把点捺写成 `n`，`4` 是 CNS 的原始写法）。
fn key(stroke: char) -> Option<char> {
    match stroke {
        '1' => Some('h'),
        '2' => Some('s'),
        '3' => Some('p'),
        '4' | 'n' => Some('n'),
        '5' => Some('z'),
        _ => None,
    }
}
