//! `gloss-gen pinyin` 写出的 JSONL：每行 `{"word":"重庆","pinyin":["chong","qing"]}`。

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use crate::error::ConvertError;

#[derive(Debug, Deserialize)]
struct Row {
    word: String,

    pinyin: Vec<String>,
}

/// 词 → 音节。同一个词以最后一条为准，坏行跳过。
pub fn load(path: &Path) -> Result<HashMap<String, Vec<String>>, ConvertError> {
    let source = std::fs::read_to_string(path)?;
    let mut out = HashMap::new();
    for line in source.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Row>(line) {
            Ok(row) => {
                out.insert(row.word, row.pinyin);
            }
            Err(error) => tracing::warn!(%error, "拼音标注坏行，跳过"),
        }
    }
    Ok(out)
}
