//! 无读音词表的导入：每行一个词（可带读音或词频），读音优先取参考词库里的词条，取不到再逐字派生。
//!
//! 面向「只有词、没有拼音」的一类词库（批量导入同学名字 / 路名 / 楼宇名这种）。派生顺序：
//!
//! 1. 词表里自己写了读音（`词<Tab>读音`）就用它——多音字词（`美露莘 mei lu xin`）只有用户知道该怎么读；
//! 2. 整词在参考词库里有词条（基础词库或随包领域词库）就用词库的读音——`建设银行` 不会派生成 `yin xing`；
//! 3. 否则从左往右**按词库做最长匹配**切分（`中华人民共和国` → `中华` + `人民` + `共和国`），
//!    每段用词库读音；切不动的单字取参考词库里词频最高的读音（`和` 取 `he`，不再出 `hu` / `huo`）。
//!
//! 一个词**只写一条读音组合**：多音字不再做笛卡尔积（`长春市朝阳区` 不会写出十几条，长词也不会因为
//! 组合数超上限被整条丢掉）。字不在参考词库里的词跳过并计入报告，不静默丢词。

use std::collections::{HashMap, HashSet};

use crate::dictionary::Dictionary;

/// 解析后的一个词。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    /// 词。
    pub text: String,

    /// 词表里写明的读音（空格分音节）；没写就是 `None`。
    pub reading: Option<String>,

    /// 词频；没写按 1。
    pub weight: u32,
}

/// 派生结果。
pub struct Derived {
    /// 青简 TSV（`词\t拼音\t词频`）。
    pub tsv: String,

    /// 派生统计。
    pub report: Report,
}

/// 派生统计。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// 写出了读音的词数（一个词一条读音）。
    pub words: usize,

    /// 跳过的词：有字不在参考词库里，推不出读音。
    pub unknown: Vec<String>,
}

/// 解析词表：每行 `词` / `词<Tab>词频` / `词<Tab>读音[<Tab>词频]`，`#` 注释、空行与 Rime 的 `---` … `...` 头忽略。
///
/// 第二列是纯数字就当词频，否则当读音（只收空格分隔的字母音节）；同一个词只留第一次出现的。
pub fn parse_words(text: &str) -> Vec<Word> {
    let mut words = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut in_header = false;
    let mut started = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // `---` … `...` 之间的 YAML 头整段跳过
        if line == "---" && !started {
            in_header = true;
            continue;
        }
        if line == "..." && in_header {
            in_header = false;
            continue;
        }
        if in_header {
            continue;
        }
        // 第一个词之前，`name:` / `sort:` 这类 YAML 键跳过（没有 `---` 的头）
        if !started && line.contains(':') && !line.contains('\t') {
            continue;
        }
        started = true;
        let mut fields = line.split('\t');
        let Some(text) = fields.next().filter(|text| !text.is_empty()) else {
            continue;
        };
        let Some((reading, weight)) = parse_tail(fields.next(), fields.next()) else {
            continue;
        };
        if seen.insert(text.to_owned()) {
            words.push(Word {
                text: text.to_owned(),
                reading,
                weight,
            });
        }
    }
    words
}

/// 第二、三列：`None` / 词频 / 读音 / 读音 + 词频。认不出（第二列既不是数字也不是读音）返回 `None`，跳过该行。
fn parse_tail(second: Option<&str>, third: Option<&str>) -> Option<(Option<String>, u32)> {
    let second = second.map(str::trim).filter(|s| !s.is_empty());
    let Some(second) = second else {
        return Some((None, 1));
    };
    if let Ok(weight) = second.parse::<u32>() {
        return Some((None, weight));
    }
    // 读音是空格分隔的字母音节，可以带数字声调（`dong1 xi5`），交给 `Dictionary::parse` 收；
    // 数字只能跟在字母后面（`zhang san 1` 这种第二列不是读音）
    let is_reading = !second.is_empty()
        && second.split(' ').all(|syllable| {
            syllable.bytes().any(|b| b.is_ascii_lowercase())
                && syllable
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        });
    if !is_reading {
        return None;
    }
    let weight = match third.map(str::trim).filter(|s| !s.is_empty()) {
        None => 1,
        Some(value) => value.parse::<u32>().ok()?,
    };
    Some((Some(second.to_owned()), weight))
}

/// 参考词库里的读音：整词词条与单字词条，各自按词频降序去重。
#[derive(Default)]
struct Readings {
    /// 词 → 读音，词频高的在前。
    words: HashMap<String, Vec<String>>,

    /// 单字 → 读音，词频高的在前。
    chars: HashMap<char, Vec<String>>,
}

impl Readings {
    fn build(references: &[&Dictionary]) -> Self {
        let mut words: HashMap<String, Vec<(u32, String)>> = HashMap::new();
        let mut chars: HashMap<char, Vec<(u32, String)>> = HashMap::new();
        for dictionary in references {
            for entry in dictionary.entries() {
                words
                    .entry(entry.text.to_owned())
                    .or_default()
                    .push((entry.frequency, entry.pinyin.to_owned()));
                let mut text_chars = entry.text.chars();
                if let (Some(ch), None) = (text_chars.next(), text_chars.next()) {
                    chars
                        .entry(ch)
                        .or_default()
                        .push((entry.frequency, entry.pinyin.to_owned()));
                }
            }
        }
        Self {
            words: sorted(words),
            chars: sorted(chars),
        }
    }

    fn word(&self, text: &str) -> Option<&[String]> {
        self.words.get(text).map(Vec::as_slice)
    }

    /// 单字词频最高的读音。
    fn char(&self, ch: char) -> Option<&str> {
        self.chars
            .get(&ch)
            .and_then(|readings| readings.first())
            .map(String::as_str)
    }

    /// 从 `text` 开头找最长的词库词条（至少两个字），返回它在字节上的长度与读音。
    fn longest(&self, text: &str) -> Option<(usize, &str)> {
        let mut best = None;
        let mut offsets = text.char_indices().skip(1).map(|(offset, _)| offset);
        for offset in offsets.by_ref() {
            if let Some(readings) = self.word(&text[..offset]) {
                best = Some((offset, readings.first()?.as_str()));
            }
        }
        // char_indices 没有「整串长度」这一项，整词命中要单独查一次
        if let Some(readings) = self.word(text) {
            best = Some((text.len(), readings.first()?.as_str()));
        }
        best
    }
}

/// 按词频降序去重成读音列表。
fn sorted<T: std::hash::Hash + Eq>(
    table: HashMap<T, Vec<(u32, String)>>,
) -> HashMap<T, Vec<String>> {
    table
        .into_iter()
        .map(|(key, mut readings)| {
            readings.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
            let mut seen = HashSet::new();
            (
                key,
                readings
                    .into_iter()
                    .map(|(_, reading)| reading)
                    .filter(|reading| seen.insert(reading.clone()))
                    .collect(),
            )
        })
        .collect()
}

/// 按参考词库给 `words` 定读音：词表自带的读音优先，其次整词词条，最后最长匹配切分加单字读音。
pub fn derive(words: &[Word], references: &[&Dictionary]) -> Derived {
    let readings = Readings::build(references);
    let mut tsv = String::new();
    let mut report = Report {
        words: 0,
        unknown: Vec::new(),
    };
    for word in words {
        // 一个词只写一条读音：词表写明的 → 整词词条（词频最高的那条）→ 最长匹配切分
        let reading = match &word.reading {
            Some(reading) => reading.clone(),
            None => match readings.word(&word.text).and_then(|known| known.first()) {
                Some(known) => known.clone(),
                None => match segment(&word.text, &readings) {
                    Some(reading) => reading,
                    None => {
                        report.unknown.push(word.text.clone());
                        continue;
                    }
                },
            },
        };
        tsv.push_str(&word.text);
        tsv.push('\t');
        tsv.push_str(&reading);
        tsv.push('\t');
        tsv.push_str(&word.weight.to_string());
        tsv.push('\n');
        report.words += 1;
    }
    Derived { tsv, report }
}

/// 最长匹配切分：词库里有整词段就用段读音，切不动的单字用词频最高的读音；有字不认识返回 `None`。
fn segment(text: &str, readings: &Readings) -> Option<String> {
    let mut rest = text;
    let mut syllables: Vec<&str> = Vec::new();
    while !rest.is_empty() {
        match readings.longest(rest) {
            Some((length, reading)) => {
                syllables.push(reading);
                rest = &rest[length..];
            }
            None => {
                let ch = rest.chars().next()?;
                syllables.push(readings.char(ch)?);
                rest = &rest[ch.len_utf8()..];
            }
        }
    }
    Some(syllables.join(" "))
}

#[cfg(test)]
mod tests;
