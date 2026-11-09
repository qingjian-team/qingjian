//! 语料统计的一元词频（`bigram` 子命令写的 lm-unigram.tsv：`词\t次数`）。

use std::collections::HashMap;
use std::path::Path;

use qingjian_dictionary::{canonical_syllable, split_tone};

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

/// `--extra-words` 文件的一行：`词\t拼音\t次数`，与词库同一列序；拼音是空格分隔的音节，可带数字声调（短语层由成分词拼出，给了就不再按字猜），
/// 留空的（`mine` 挖出来的词）按字猜。
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
            let pinyin = fields.next()?;
            let count = fields.next()?.trim().parse().ok()?;
            let syllables = Some(
                pinyin
                    .split_whitespace()
                    .map(|s| canonical_syllable(split_tone(s).0).to_owned())
                    .collect::<Vec<_>>(),
            )
            .filter(|s| !s.is_empty());
            Some(ExtraWord {
                text,
                count,
                syllables,
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_given_pinyin() {
        let path = std::env::temp_dir().join(format!(
            "qingjian-extra-words-normalize-{}.tsv",
            std::process::id()
        ));
        std::fs::write(&path, "策略\tce lue\t20\n虐待\tnve4 dai4\t10\n回家\t\t30\n").unwrap();
        let words = extra_words(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(words[0].syllables.as_deref().unwrap(), ["ce", "lve"]);
        assert_eq!(words[0].count, 20);
        assert_eq!(words[1].syllables.as_deref().unwrap(), ["nve", "dai"]);
        assert_eq!(words[2].syllables, None);
        assert_eq!(words[2].count, 30);
    }
}
