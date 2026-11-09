//! 从外部词表补出词库没有的词：知意三元模型（`zh.ngram`）的词表里有不少网络用语、专名与技术词（摸鱼、七神、老梗），
//! 青简的词库都没收，整句只能逐字拼。这里按用户机器上已有的那份词表在本机生成补充词库，不随包分发派生数据。
//!
//! 读音按参照词库里每个字的读音统计猜（词里的读音比单字的权重大），占比够高的次要读音也收；
//! 词频按两边都有的词在对数坐标拟直线，从词表的一元概率换算到主词库的尺度。

use std::collections::{HashMap, HashSet};

use crate::{Dictionary, DictionaryError};

/// 补的词最多几个字（与整句词图一个词最多跨的音节数一致）。
const MAX_CHARS: usize = 8;

/// 次要读音占这个字全部读音的比例到这么多才另收一条（`行` 的 xing / hang）。
const MINOR_READING_SHARE: f64 = 0.25;

/// 一个词最多收几种读音组合：多音字多的词组合会爆，后面的读音多半是错的。
const MAX_READINGS: usize = 4;

/// 两边都有的词少于这么多条时不拟（词表或参照词库不对），一个都不补。
const MIN_FIT_PAIRS: usize = 100;

/// 词里的读音比单字条目的权重大这么多倍：单字的词频含着它所有的用法，词里的读音更接近它在多字词里的读法。
const WORD_READING_WEIGHT: u64 = 4;

/// `words` 是 (词, 一元 log10 P)；`references` 里有的词不补，能完整切成参照词库里多字词的（进行分析 = 进行 + 分析）也不补，
/// 那是词表的分词单位，不是缺的词。第一本参照词库是主词库，词频按它的尺度换算。
pub fn supplement<'a>(
    words: impl Iterator<Item = (&'a str, f64)>,
    references: &[&Dictionary],
) -> Result<Dictionary, DictionaryError> {
    let Some(primary) = references.first() else {
        return Dictionary::parse("");
    };
    let mut known: HashSet<&str> = HashSet::new();
    let mut readings: HashMap<char, HashMap<&str, u64>> = HashMap::new();
    for dictionary in references {
        for entry in dictionary.entries() {
            known.insert(entry.text);
            let syllables: Vec<&str> = entry.pinyin.split(' ').collect();
            if syllables.len() != entry.text.chars().count() {
                continue;
            }
            let weight = if syllables.len() == 1 {
                1
            } else {
                WORD_READING_WEIGHT
            };
            for (ch, syllable) in entry.text.chars().zip(syllables) {
                *readings.entry(ch).or_default().entry(syllable).or_default() +=
                    weight * (u64::from(entry.frequency) + 1);
            }
        }
    }
    let mut frequencies: HashMap<&str, u32> = HashMap::new();
    for entry in primary.entries() {
        let slot = frequencies.entry(entry.text).or_default();
        *slot = (*slot).max(entry.frequency);
    }
    let words: Vec<(&str, f64)> = words
        .filter(|(text, _)| {
            let count = text.chars().count();
            (2..=MAX_CHARS).contains(&count) && text.chars().all(is_han)
        })
        .collect();
    let pairs: Vec<(f64, f64)> = words
        .iter()
        .filter_map(|(text, log10)| {
            let frequency = *frequencies.get(text)?;
            (frequency > 0).then(|| (*log10, f64::from(frequency).ln()))
        })
        .collect();
    let Some((intercept, slope)) = fit(&pairs) else {
        tracing::warn!(pairs = pairs.len(), "词表与主词库重合太少，不补词");
        return Dictionary::parse("");
    };
    let mut source = String::new();
    let mut added = 0usize;
    for (text, log10) in &words {
        if known.contains(text) || splits_into_words(text, &known) {
            continue;
        }
        let frequency = (intercept + slope * log10)
            .exp()
            .round()
            .clamp(1.0, f64::from(u32::MAX));
        let readings = guess_readings(text, &readings);
        for reading in &readings {
            source.push_str(&format!("{text}\t{reading}\t{frequency}\n"));
        }
        added += usize::from(!readings.is_empty());
    }
    tracing::info!(words = added, slope, intercept, "从词表补出的词");
    Dictionary::parse(&source)
}

/// 最小二乘拟 ln 词频 = 截距 + 斜率 · log10 P，返回 (截距, 斜率)。
fn fit(pairs: &[(f64, f64)]) -> Option<(f64, f64)> {
    if pairs.len() < MIN_FIT_PAIRS {
        return None;
    }
    let n = pairs.len() as f64;
    let mean_x = pairs.iter().map(|p| p.0).sum::<f64>() / n;
    let mean_y = pairs.iter().map(|p| p.1).sum::<f64>() / n;
    let spread: f64 = pairs.iter().map(|p| (p.0 - mean_x).powi(2)).sum();
    if spread <= 0.0 {
        return None;
    }
    let slope = pairs
        .iter()
        .map(|p| (p.0 - mean_x) * (p.1 - mean_y))
        .sum::<f64>()
        / spread;
    Some((mean_y - slope * mean_x, slope))
}

/// 整个词能不能切成参照词库里的多字词（整个词本身不在 `known` 里才问，所以至少两段）。
fn splits_into_words(text: &str, known: &HashSet<&str>) -> bool {
    let bounds: Vec<usize> = text
        .char_indices()
        .map(|(i, _)| i)
        .chain([text.len()])
        .collect();
    // reach[i]：前 i 个字能切成多字词
    let mut reach = vec![false; bounds.len()];
    reach[0] = true;
    for end in 2..bounds.len() {
        reach[end] = (0..end - 1)
            .any(|start| reach[start] && known.contains(&text[bounds[start]..bounds[end]]));
    }
    reach[bounds.len() - 1]
}

/// 每个字的主读音，加上占比够高的次要读音，组合至多 [`MAX_READINGS`] 种；有字查不到读音就一种都不给。
fn guess_readings(text: &str, readings: &HashMap<char, HashMap<&str, u64>>) -> Vec<String> {
    let mut combinations = vec![String::new()];
    for ch in text.chars() {
        let Some(counts) = readings.get(&ch) else {
            return Vec::new();
        };
        let total: u64 = counts.values().sum();
        let mut options: Vec<(&str, u64)> = counts.iter().map(|(s, c)| (*s, *c)).collect();
        options.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        let kept: Vec<&str> = options
            .iter()
            .enumerate()
            .filter(|(i, (_, count))| {
                *i == 0 || *count as f64 >= MINOR_READING_SHARE * total as f64
            })
            .map(|(_, (syllable, _))| *syllable)
            .collect();
        combinations = combinations
            .iter()
            .flat_map(|prefix| {
                kept.iter().map(move |syllable| {
                    if prefix.is_empty() {
                        (*syllable).to_owned()
                    } else {
                        format!("{prefix} {syllable}")
                    }
                })
            })
            .take(MAX_READINGS)
            .collect();
    }
    combinations
}

fn is_han(ch: char) -> bool {
    matches!(ch, '\u{4e00}'..='\u{9fff}' | '\u{3400}'..='\u{4dbf}')
}

#[cfg(test)]
mod tests;
