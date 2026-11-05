//! 名字的读音：先看开头是不是多音字姓，再看整个名字是不是词库里的词（爸爸、老婆这类备注名），都不是就逐字取最常见的读音。

use super::surname::surname;
use crate::zhuyin::tone;

/// 整词读音最多试几种多音组合：名字短，够用；多了是一串多音字，逐字的常见读音也就够了。
const MAX_COMBINATIONS: usize = 32;

/// `name` 的读音（空格分音节），一般一条；按姓读的与词库里整词的读音不同时两条都给，姓的在前。
/// `word` 问某组音节下词库里有没有这个名字；`fallback` 给单字表里没有的字一个读音。有字读不出来时为空。
pub(super) fn readings(
    name: &str,
    mut word: impl FnMut(&[&str]) -> bool,
    mut fallback: impl FnMut(char) -> Option<String>,
) -> Vec<String> {
    let mut options: Vec<Vec<String>> = Vec::new();
    for ch in name.chars() {
        let known = tone::readings(ch);
        let readings: Vec<String> = if known.is_empty() {
            fallback(ch).into_iter().collect()
        } else {
            known.into_iter().map(str::to_owned).collect()
        };
        if readings.is_empty() {
            return Vec::new();
        }
        options.push(readings);
    }
    let usual: Vec<&str> = options.iter().map(|o| o[0].as_str()).collect();
    let whole = combinations(&options)
        .into_iter()
        .find(|syllables| word(syllables));
    let by_surname = surname(name).map(|(chars, reading)| {
        let rest = match &whole {
            Some(whole) => &whole[chars..],
            None => &usual[chars..],
        };
        std::iter::once(reading)
            .chain(rest.iter().copied())
            .collect::<Vec<_>>()
            .join(" ")
    });
    let whole = whole.map(|syllables| syllables.join(" "));
    let mut out: Vec<String> = by_surname.into_iter().chain(whole).collect();
    out.dedup();
    if out.is_empty() {
        out.push(usual.join(" "));
    }
    out
}

/// 各字读音的组合，常见读音靠前（逐位按读音顺序展开），最多 [`MAX_COMBINATIONS`] 种。
fn combinations(options: &[Vec<String>]) -> Vec<Vec<&str>> {
    let mut out: Vec<Vec<&str>> = vec![Vec::new()];
    for readings in options {
        let mut next = Vec::new();
        for prefix in &out {
            for reading in readings {
                if next.len() == MAX_COMBINATIONS {
                    break;
                }
                let mut syllables = prefix.clone();
                syllables.push(reading.as_str());
                next.push(syllables);
            }
        }
        out = next;
    }
    out.sort_by_key(|syllables| {
        syllables
            .iter()
            .zip(options)
            .filter(|(s, o)| **s != o[0])
            .count()
    });
    out
}
