//! Unicode CLDR emoji annotations（中文或英文）→ `词\temoji …`。
//!
//! JSON 结构：`{"annotations" | "annotationsDerived": {"annotations": {"😀": {"default": [关键词…], "tts": [名字]}}}}`。
//! 名字正好是这个词的排在前面，其余按「名字里含这个词」再按关键词位置排（中文关键词大致按相关度列，英文是字母序，所以名字命中更重要）；
//! 单字词（爱、小）只收名字里含这个字的 emoji，否则 爱 会配出 👨‍🍼 这种只沾边的。
//! 跳过肤色修饰符、单独的 ASCII 字符、几何形状和带肤色的变体（表会大三倍，候选里也用不上）。

use std::collections::BTreeMap;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::ConvertError;

/// 关键词命中：(emoji, 这个词在它关键词表里的位置, 它的名字里含这个词, 含这个词的最短名字长度)。
/// 名字越短越基本（red heart 比 heart with arrow 更该排前）。
type KeywordHit = (String, usize, bool, usize);

pub fn convert(inputs: &[PathBuf], output: &Path, language: &str) -> Result<(), ConvertError> {
    // 词 → (名字命中的 emoji, 关键词命中)
    let mut table: BTreeMap<String, (Vec<String>, Vec<KeywordHit>)> = BTreeMap::new();
    let mut emojis = 0;
    for path in inputs {
        let json: Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;
        let root = json
            .get("annotations")
            .or_else(|| json.get("annotationsDerived"))
            .and_then(|v| v.get("annotations"))
            .and_then(Value::as_object)
            .ok_or_else(|| ConvertError::Format {
                path: path.clone(),
                line: 0,
                reason: "expected annotations.annotations object".to_owned(),
            })?;
        for (emoji, entry) in root {
            if !keep(emoji) {
                continue;
            }
            emojis += 1;
            let words = |key: &str| -> Vec<String> {
                entry
                    .get(key)
                    .and_then(Value::as_array)
                    .map(|list| {
                        list.iter()
                            .filter_map(Value::as_str)
                            .map(str::trim)
                            .filter(|w| !w.is_empty() && !w.contains(' '))
                            .map(str::to_owned)
                            .collect()
                    })
                    .unwrap_or_default()
            };
            let names = words("tts");
            for name in &names {
                table.entry(name.clone()).or_default().0.push(emoji.clone());
            }
            // 「名字里含这个词」按完整名字判断：英文名字多是短语（grinning face），不能只看单词名
            let full_names: Vec<String> = entry
                .get("tts")
                .and_then(Value::as_array)
                .map(|list| {
                    list.iter()
                        .filter_map(Value::as_str)
                        .map(|s| s.trim().to_lowercase())
                        .collect()
                })
                .unwrap_or_default();
            for (position, keyword) in words("default").into_iter().enumerate() {
                let keyword = keyword.to_lowercase();
                // 英文关键词与名字里的变形要对得上（smile / smiling）：去掉词尾 e 再找
                let stem = keyword.strip_suffix('e').unwrap_or(&keyword);
                let name_len = full_names
                    .iter()
                    .filter(|n| {
                        n.contains(keyword.as_str()) || (stem.len() >= 3 && n.contains(stem))
                    })
                    .map(|n| n.chars().count())
                    .min();
                table.entry(keyword).or_default().1.push((
                    emoji.clone(),
                    position,
                    name_len.is_some(),
                    name_len.unwrap_or(usize::MAX),
                ));
            }
        }
        tracing::info!(path = %path.display(), "已读取");
    }
    let mut file = BufWriter::new(std::fs::File::create(output)?);
    writeln!(
        file,
        "# 由 qingjian-dict-convert 从 Unicode CLDR {language} annotations 生成（Unicode License v3）。词\\temoji emoji …"
    )?;
    let mut words = 0;
    for (word, (named, mut keyed)) in table {
        let single = word.chars().count() == 1;
        // 单字词只要名字里含这个字的；名字里含词的靠前（名字越短越基本），再按关键词位置，老符号（U+1F000 以下）最后
        keyed.retain(|(_, _, in_name, _)| !single || *in_name);
        keyed.sort_by_key(|(emoji, position, in_name, name_len)| {
            (!*in_name, *name_len, *position, legacy(emoji))
        });
        let mut list: Vec<&str> = Vec::new();
        for emoji in named.iter().chain(keyed.iter().map(|(e, _, _, _)| e)) {
            if !list.contains(&emoji.as_str()) {
                list.push(emoji);
            }
        }
        if list.is_empty() {
            continue;
        }
        words += 1;
        writeln!(file, "{word}\t{}", list.join(" "))?;
    }
    file.flush()?;
    tracing::info!(path = %output.display(), emojis, words, "写出完成");
    Ok(())
}

/// 要不要收这个 emoji：跳过纯 ASCII、肤色修饰符本身、带肤色修饰符的变体、几何形状（▪ ▫ ◾ 这类当候选只是噪音）。
fn keep(emoji: &str) -> bool {
    if emoji.is_ascii() {
        return false;
    }
    !emoji
        .chars()
        .any(|c| ('\u{1F3FB}'..='\u{1F3FF}').contains(&c) || ('\u{25A0}'..='\u{25FF}').contains(&c))
}

/// 不在 emoji 专属区（U+1F000 以上）的老符号。
fn legacy(emoji: &str) -> bool {
    emoji.chars().next().is_none_or(|c| (c as u32) < 0x1F000)
}
