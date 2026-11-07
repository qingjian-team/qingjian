//! 拆字表：開放詞典網的拆字字典（简体 + 繁体，CC BY 3.0）+ 青简词库的单字读音与字频 + 繁简索引
//! → `chaizi.tsv`（`字\t读音\t拆法部件\t拆法读音`），编进 Core 给问字模式按部件读音或笔画找字。
//!
//! 第 3 列是部件本身（`林` 写成 `木木`），Core 拿它查每个部件自己的笔顺，于是「部件读音与笔画混着打」能成立；
//! 第 4 列是每个部件的读音，与第 3 列一一对应：部件之间 `'`，同一个部件的多个读音用 `,`（不再交叉展开）。
//! 繁体字典里「简体表没有的字」按繁简索引落到简体主键（`謝` → `谢`），拆法并进同一个字——
//! 打台北字形的拆法（`辶 言`）也能找到简体字，输出侧照旧由 Core 的 `Traditional` 在出口转。
//!
//! 读音为空的部件留空串（`'` 两侧空着）：Core 对这种部件只走笔顺，不影响同一拆法的其他部件。
//! 行按字频从高到低排，Core 按行序出候选；没有读音的字整行跳过（拿不到字频，也标不出读音）。

use std::collections::{HashMap, HashSet};
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;

use crate::error::ConvertError;
use crate::stroke::single_char;

mod options;

#[cfg(test)]
mod tests;

pub use options::ChaiziOptions;

/// 一个部件最多留几个读音参与匹配（行 xing / hang / heng 这种）。
const READINGS_PER_PART: usize = 3;

/// 生成结果的统计。
pub struct Summary {
    /// 写出的字数。
    pub characters: usize,

    /// 写出的拆法总数。
    pub splits: usize,

    /// 词库里查不到读音、整行跳过的字数。
    pub unread: usize,

    /// 繁体表里落到简体主键的字数。
    pub merged: usize,
}

/// 一个字的一行：读音、字频与各种拆法。
struct Row {
    ch: char,
    frequency: u64,
    syllables: Vec<String>,
    splits: Vec<Split>,
}

/// 一种拆法：部件本身，与每个部件的读音（可能为空）。
struct Split {
    parts: Vec<char>,
    readings: Vec<Vec<String>>,
}

impl Split {
    /// 部件连写（`木木`）。
    fn parts_text(&self) -> String {
        self.parts.iter().collect()
    }

    /// 部件读音：部件之间 `'`，同一部件的多个读音用 `,`；读不出的部件留空。
    fn readings_text(&self) -> String {
        self.readings
            .iter()
            .map(|list| list.join(","))
            .collect::<Vec<_>>()
            .join("'")
    }
}

/// 繁简索引：繁体 → 简体。表里没有的字原样返回。
struct Simplified {
    map: HashMap<char, char>,
}

impl Simplified {
    fn from_path(path: &Path) -> Result<Self, ConvertError> {
        let mut map = HashMap::new();
        for line in std::fs::read_to_string(path)?.lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            let (Some(from), Some(to)) = (
                fields.next().and_then(single_char),
                fields.next().and_then(single_char),
            ) else {
                continue;
            };
            if from != to {
                map.entry(from).or_insert(to);
            }
        }
        Ok(Self { map })
    }

    /// 繁体落到简体；本来就是简体（或表里没有）原样返回。
    fn simplify(&self, ch: char) -> char {
        self.map.get(&ch).copied().unwrap_or(ch)
    }
}

/// 读拆字字典与词库，写拆字表。
pub fn convert(options: &ChaiziOptions, out_dir: &Path) -> Result<Summary, ConvertError> {
    let readings = char_readings(&std::fs::read_to_string(&options.dictionary)?);
    let simplified = Simplified::from_path(&options.fanjian)?;

    let mut rows: Vec<Row> = Vec::new();
    let mut row_of: HashMap<char, usize> = HashMap::new();
    let mut plain: HashSet<char> = HashSet::new();
    let mut unread = 0;
    let mut merged = 0;
    for (index, path) in options.chaizi.iter().enumerate() {
        // 第一张是简体表：它的字一律原样，其余表只补简体表没有的字
        let base = index == 0;
        let source = std::fs::read_to_string(path)?;
        for line in source.lines() {
            let mut fields = line.split('\t');
            let Some(ch) = fields.next().and_then(single_char) else {
                continue;
            };
            // 拆字字典里混着 □ 这类方框符号（用作部件时靠 parts.tsv 认），它们不是字，不出候选
            if !is_han(ch) {
                continue;
            }
            if base {
                plain.insert(ch);
            } else if plain.contains(&ch) {
                // 简体表已经有了，繁体字形与它完全同形，拆法不重复收
                continue;
            }
            // 繁简索引里没有简体对应的字写成 `□` 占位，这种退回原字，不要整个丢掉
            let mut key = if base { ch } else { simplified.simplify(ch) };
            if !is_han(key) {
                key = ch;
            }
            if !base && key != ch {
                merged += 1;
            }
            let Some(own) = readings.get(&key).or_else(|| readings.get(&ch)) else {
                unread += 1;
                continue;
            };
            let frequency: u64 = own.iter().map(|(_, count)| count).sum();
            let target = match row_of.get(&key) {
                Some(&row) => row,
                None => {
                    row_of.insert(key, rows.len());
                    rows.push(Row {
                        ch: key,
                        frequency: 0,
                        syllables: Vec::new(),
                        splits: Vec::new(),
                    });
                    rows.len() - 1
                }
            };
            let row = &mut rows[target];
            row.frequency = row.frequency.max(frequency);
            for (syllable, _) in own {
                if !row.syllables.contains(syllable) {
                    row.syllables.push(syllable.clone());
                }
            }
            for split in fields {
                let Some(one) = split_of(split, &readings) else {
                    continue;
                };
                if !row.splits.iter().any(|known| known.parts == one.parts) {
                    row.splits.push(one);
                }
            }
        }
    }
    rows.sort_by(|a, b| b.frequency.cmp(&a.frequency).then(a.ch.cmp(&b.ch)));

    let mut out = BufWriter::new(std::fs::File::create(out_dir.join("chaizi.tsv"))?);
    writeln!(
        out,
        "# 拆字表，由 qingjian-dict-convert chaizi 从 assets/chaizi（開放詞典網拆字字典，CC BY 3.0）\
         与青简词库生成。字\t读音\t拆法部件\t拆法读音（部件间 '，同一部件多读音用 ,）"
    )?;
    let mut splits = 0;
    for row in &rows {
        if row.splits.is_empty() {
            continue;
        }
        splits += row.splits.len();
        let parts: Vec<String> = row.splits.iter().map(Split::parts_text).collect();
        let reading: Vec<String> = row.splits.iter().map(Split::readings_text).collect();
        writeln!(
            out,
            "{}\t{}\t{}\t{}",
            row.ch,
            row.syllables.join(" "),
            parts.join(" "),
            reading.join(" ")
        )?;
    }
    out.flush()?;
    Ok(Summary {
        characters: rows.iter().filter(|row| !row.splits.is_empty()).count(),
        splits,
        unread,
        merged,
    })
}

/// 一种拆法（`言 射`）里的部件与每个部件的读音；一个部件都认不出时返回 `None`。
fn split_of(split: &str, readings: &HashMap<char, Vec<(String, u64)>>) -> Option<Split> {
    let mut parts = Vec::new();
    let mut lists = Vec::new();
    for part in split.split(' ') {
        let ch = single_char(part)?;
        parts.push(ch);
        lists.push(
            readings
                .get(&ch)
                .map(|options| {
                    options
                        .iter()
                        .take(READINGS_PER_PART)
                        .map(|(syllable, _)| syllable.clone())
                        .collect()
                })
                .unwrap_or_default(),
        );
    }
    (!parts.is_empty()).then_some(Split {
        parts,
        readings: lists,
    })
}

/// 是不是汉字：只留 CJK 各平面的字，方框、箭头这类符号不出候选。
fn is_han(ch: char) -> bool {
    matches!(ch,
        '\u{3400}'..='\u{4DBF}'
        | '\u{4E00}'..='\u{9FFF}'
        | '\u{F900}'..='\u{FAFF}'
        | '\u{20000}'..='\u{2FA1F}'
    )
}

/// 词库里每个单字的读音（去掉数字声调）与词频，读音按词频从高到低。
fn char_readings(dictionary: &str) -> HashMap<char, Vec<(String, u64)>> {
    let mut readings: HashMap<char, Vec<(String, u64)>> = HashMap::new();
    for line in dictionary.lines().filter(|line| !line.starts_with('#')) {
        let mut fields = line.split('\t');
        let (Some(word), Some(pinyin)) = (fields.next(), fields.next()) else {
            continue;
        };
        let Some(ch) = single_char(word) else {
            continue;
        };
        let syllable: String = pinyin.chars().filter(char::is_ascii_lowercase).collect();
        if syllable.is_empty() {
            continue;
        }
        let count = fields.next().and_then(|f| f.parse().ok()).unwrap_or(0);
        let list = readings.entry(ch).or_default();
        match list.iter_mut().find(|(known, _)| *known == syllable) {
            Some((_, total)) => *total += count,
            None => list.push((syllable, count)),
        }
    }
    for list in readings.values_mut() {
        list.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    }
    readings
}
