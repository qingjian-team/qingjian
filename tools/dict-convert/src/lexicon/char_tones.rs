//! 单字声调表：从通用词表（`wei4'shen2'me` 这种带数字声调的读音）逐字收「音节 + 声调」，
//! 写成 `字\tma1 ma5 …`，编进 Core 给注音按敲的声调排序（`qingjian_core::zhuyin::tone`）。
//!
//! 词库本身不带声调（`lexicon` 把声调去掉了），所以单独出这张表；轻声记 5，同一个字的读音按首次出现的顺序。

use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::path::Path;

use qingjian_core::parser::is_syllable;

use super::tone::strip_tone;
use crate::error::ConvertError;

/// 读通用词表（`word\tpinyin\t…`，首行表头）写声调表，返回收到的字数。
pub fn convert(words: &Path, output: &Path) -> Result<usize, ConvertError> {
    let source = std::fs::read_to_string(words)?;
    let mut order: Vec<char> = Vec::new();
    let mut readings: HashMap<char, Vec<String>> = HashMap::new();
    for line in source.lines().skip(1) {
        let mut fields = line.split('\t');
        let (Some(word), Some(pinyin)) = (fields.next(), fields.next()) else {
            continue;
        };
        let syllables: Vec<&str> = pinyin.split('\'').map(str::trim).collect();
        // 字数与音节数对不上（带儿化、字母词）的整条跳过，免得错位
        if syllables.len() != word.chars().count() {
            continue;
        }
        for (ch, syllable) in word.chars().zip(syllables) {
            let Some(reading) = toned(syllable) else {
                continue;
            };
            let list = readings.entry(ch).or_insert_with(|| {
                order.push(ch);
                Vec::new()
            });
            if !list.contains(&reading) {
                list.push(reading);
            }
        }
    }
    let mut out = BufWriter::new(std::fs::File::create(output)?);
    writeln!(
        out,
        "# 单字声调表，由 qingjian-dict-convert tones 从 assets/lexicon/02_common 的通用词表生成。字\t带调音节（轻声记 5）"
    )?;
    for ch in &order {
        writeln!(out, "{ch}\t{}", readings[ch].join(" "))?;
    }
    out.flush()?;
    Ok(order.len())
}

/// `ma1` → `ma1`，`ma` → `ma5`；不是拼音音节的返回 None。
fn toned(syllable: &str) -> Option<String> {
    let tone = match syllable.chars().last()? {
        c @ '1'..='5' => c,
        _ => '5',
    };
    let plain = strip_tone(syllable);
    is_syllable(&plain).then(|| format!("{plain}{tone}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_and_umlaut_are_normalized() {
        assert_eq!(toned("ma1").as_deref(), Some("ma1"));
        assert_eq!(toned("ma").as_deref(), Some("ma5"));
        assert_eq!(toned("lv4").as_deref(), Some("lv4"));
        assert_eq!(toned("xyz3"), None);
    }

    #[test]
    fn collects_readings_per_character() {
        let dir = std::env::temp_dir().join("qingjian-char-tones-test");
        std::fs::create_dir_all(&dir).unwrap();
        let words = dir.join("words.tsv");
        let output = dir.join("tones.tsv");
        std::fs::write(
            &words,
            "word\tpinyin\trank\n吗\tma\t1\n妈妈\tma1'ma1\t2\n吗\tma2\t3\n这儿\tzhe4\t4\n",
        )
        .unwrap();
        assert_eq!(convert(&words, &output).unwrap(), 2);
        let text = std::fs::read_to_string(&output).unwrap();
        let body: Vec<&str> = text.lines().skip(1).collect();
        assert_eq!(body, ["吗\tma5 ma2", "妈\tma1"]);
    }
}
