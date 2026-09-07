//! 语料统计的一元词频（`bigram` 子命令写的 lm-unigram.tsv：`词\t次数`）。

use std::collections::HashMap;
use std::path::Path;

use crate::error::ConvertError;

pub fn load(path: &Path) -> Result<HashMap<String, u64>, ConvertError> {
    Ok(rows(path)?.into_iter().collect())
}

/// 同上，保持文件顺序、允许重复。
pub fn rows(path: &Path) -> Result<Vec<(String, u64)>, ConvertError> {
    let source = std::fs::read_to_string(path)?;
    Ok(source
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let (word, count) = l.split_once('\t')?;
            Some((word.trim().to_owned(), count.trim().parse().ok()?))
        })
        .collect())
}
