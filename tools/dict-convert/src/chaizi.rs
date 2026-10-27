//! 拆字表：開放詞典網的拆字字典（`assets/chaizi/chaizi-jt.txt`，CC BY 3.0）+ 青简词库的单字读音与字频
//! → `chaizi.tsv`（`字\t读音…\t部件拼音串…`），编进 Core 给问字模式按部件读音找字（`umumu` → 林）。
//!
//! 部件拼音串是一种拆法里各部件读音连写（林 = `mumu`）；部件多音的按词频取前几个读音交叉组合，
//! 有部件读不出来（`㇆` 这类笔画符号）的那种拆法跳过。行按字频从高到低排，Core 按行序出候选。

use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::error::ConvertError;

/// 一个部件最多取几个读音参与组合（行 xing / hang / heng 这种）。
const READINGS_PER_PART: usize = 3;

/// 一种拆法最多出几个拼音串，免得多音部件多的拆法组合爆炸。
const KEYS_PER_SPLIT: usize = 16;

/// 生成结果的统计。
pub struct Summary {
    /// 写出的字数。
    pub characters: usize,

    /// 词库里查不到读音、整个跳过的字数。
    pub unread: usize,
}

/// 读拆字字典与词库（`词\t音节\t词频`），写拆字表。
pub fn convert(chaizi: &Path, dictionary: &Path, output: &Path) -> Result<Summary, ConvertError> {
    let readings = char_readings(&std::fs::read_to_string(dictionary)?);
    let mut rows: Vec<(u64, char, Vec<String>, Vec<String>)> = Vec::new();
    let mut row_of: HashMap<char, usize> = HashMap::new();
    let mut unread = 0;
    for line in std::fs::read_to_string(chaizi)?.lines() {
        let mut fields = line.split('\t');
        let Some(ch) = fields.next().and_then(single_char) else {
            continue;
        };
        let Some(own) = readings.get(&ch) else {
            unread += 1;
            continue;
        };
        let mut keys: Vec<String> = Vec::new();
        for split in fields {
            for key in split_keys(split, &readings) {
                if !keys.contains(&key) {
                    keys.push(key);
                }
            }
        }
        if keys.is_empty() {
            continue;
        }
        // 原表有同一个字分两行写的（了），拆法并到先出现的那行
        if let Some(&row) = row_of.get(&ch) {
            let known = &mut rows[row].3;
            for key in keys {
                if !known.contains(&key) {
                    known.push(key);
                }
            }
            continue;
        }
        let frequency = own.iter().map(|(_, count)| count).sum();
        let syllables = own.iter().map(|(syllable, _)| syllable.clone()).collect();
        row_of.insert(ch, rows.len());
        rows.push((frequency, ch, syllables, keys));
    }
    rows.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));

    let mut out = BufWriter::new(std::fs::File::create(output)?);
    writeln!(
        out,
        "# 拆字表，由 qingjian-dict-convert chaizi 从 assets/chaizi（開放詞典網拆字字典，CC BY 3.0）与青简词库生成。字\t读音\t部件拼音串"
    )?;
    for (_, ch, syllables, keys) in &rows {
        writeln!(out, "{ch}\t{}\t{}", syllables.join(" "), keys.join(" "))?;
    }
    out.flush()?;
    Ok(Summary {
        characters: rows.len(),
        unread,
    })
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

/// 一种拆法（`言 射`）的各部件读音连写；有部件读不出来返回空。
fn split_keys(split: &str, readings: &HashMap<char, Vec<(String, u64)>>) -> Vec<String> {
    let mut keys = vec![String::new()];
    for part in split.split(' ') {
        let Some(options) = single_char(part).and_then(|ch| readings.get(&ch)) else {
            return Vec::new();
        };
        keys = keys
            .iter()
            .flat_map(|key| {
                options
                    .iter()
                    .take(READINGS_PER_PART)
                    .map(move |(syllable, _)| format!("{key}{syllable}"))
            })
            .take(KEYS_PER_SPLIT)
            .collect();
    }
    keys
}

fn single_char(text: &str) -> Option<char> {
    let mut chars = text.chars();
    let ch = chars.next()?;
    chars.next().is_none().then_some(ch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_join_part_readings_and_skip_unreadable_parts() {
        let readings = char_readings("木\tmu\t100\n行\txing\t90\n行\thang\t10\n言\tyan\t50\n");
        assert_eq!(split_keys("木 木", &readings), ["mumu"]);
        assert_eq!(split_keys("木 行", &readings), ["muxing", "muhang"]);
        assert!(split_keys("木 ㇆", &readings).is_empty());
    }
}
