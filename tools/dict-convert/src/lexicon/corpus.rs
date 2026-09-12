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

/// `--extra-words` 文件的一行：`词\t次数[\t拼音]`，拼音是空格分隔的音节（短语层由成分词拼出，给了就不再按字猜）。
pub struct ExtraWord {
    /// 词。
    pub text: String,

    /// 语料次数，当词频。
    pub count: u64,

    /// 读音；没给为 `None`。
    pub syllables: Option<Vec<String>>,
}

/// 读额外词文件，保持顺序。
pub fn extra_words(path: &Path) -> Result<Vec<ExtraWord>, ConvertError> {
    let source = std::fs::read_to_string(path)?;
    Ok(source
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let mut fields = l.split('\t');
            let text = fields.next()?.trim().to_owned();
            let count = fields.next()?.trim().parse().ok()?;
            let syllables = fields
                .next()
                .map(|p| p.split_whitespace().map(str::to_owned).collect::<Vec<_>>())
                .filter(|s| !s.is_empty());
            Some(ExtraWord {
                text,
                count,
                syllables,
            })
        })
        .collect())
}
