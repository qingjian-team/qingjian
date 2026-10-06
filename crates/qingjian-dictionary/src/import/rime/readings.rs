//! 从显式注音字表生成无注音词的读音；按 Rime 的 5% 字频门槛过滤罕见读音。

use std::collections::HashMap;

use super::entry::Entry;

/// 避免多音字组合膨胀；超限时整词跳过，不留下依赖枚举顺序的部分读音。
const MAX_READINGS: usize = 256;

pub(super) fn collect(entries: &[Entry]) -> HashMap<char, Vec<&str>> {
    let mut weighted: HashMap<char, Vec<(&str, u32)>> = HashMap::new();
    for entry in entries {
        let mut chars = entry.word.chars();
        let Some(ch) = chars.next() else { continue };
        if chars.next().is_some()
            || entry.code.is_empty()
            || !entry.code.bytes().all(|b| b.is_ascii_lowercase())
        {
            continue;
        }
        weighted
            .entry(ch)
            .or_default()
            .push((&entry.code, entry.weight));
    }
    weighted
        .into_iter()
        .map(|(ch, codes)| {
            let total: u64 = codes.iter().map(|(_, weight)| u64::from(*weight)).sum();
            let codes = codes
                .into_iter()
                .filter(|(_, weight)| u64::from(*weight) * 20 >= total)
                .map(|(code, _)| code)
                .collect();
            (ch, codes)
        })
        .collect()
}

pub(super) fn encode(word: &str, readings: &HashMap<char, Vec<&str>>) -> Option<Vec<String>> {
    let mut codes = vec![String::new()];
    for ch in word.chars() {
        let choices = readings.get(&ch)?;
        if choices.is_empty() || codes.len().checked_mul(choices.len())? > MAX_READINGS {
            return None;
        }
        codes = codes
            .into_iter()
            .flat_map(|prefix| {
                choices.iter().map(move |code| {
                    if prefix.is_empty() {
                        (*code).to_owned()
                    } else {
                        format!("{prefix} {code}")
                    }
                })
            })
            .collect();
    }
    Some(codes)
}
