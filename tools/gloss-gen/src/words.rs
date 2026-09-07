//! 从一元词频表里挑要生成释义的词。

use std::path::Path;

use crate::error::GlossError;

/// 读 `词\t次数`（lm-unigram.tsv）或 `词\t拼音\t词频`（dict.tsv，取最后一列、同词多读音取最大），
/// 只留全是汉字、不超过 `max_chars` 个字、次数不低于 `min_count` 的词，按次数降序，最多 `limit` 个。
pub fn select(
    path: &Path,
    min_count: u64,
    max_chars: usize,
    limit: Option<usize>,
) -> Result<Vec<String>, GlossError> {
    let source = std::fs::read_to_string(path)?;
    let mut best: std::collections::HashMap<&str, u64> = std::collections::HashMap::new();
    for line in source.lines() {
        if line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let Some(word) = fields.next() else { continue };
        let Some(count) = fields
            .next_back()
            .and_then(|c| c.trim().parse::<u64>().ok())
        else {
            continue;
        };
        let slot = best.entry(word).or_insert(0);
        *slot = (*slot).max(count);
    }
    let mut words: Vec<(String, u64)> = best
        .into_iter()
        .filter_map(|(word, count)| {
            let chars = word.chars().count();
            (count >= min_count && (1..=max_chars).contains(&chars) && word.chars().all(is_han))
                .then(|| (word.to_owned(), count))
        })
        .collect();
    words.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    if let Some(limit) = limit {
        words.truncate(limit);
    }
    if words.is_empty() {
        return Err(GlossError::NoWords(path.to_owned()));
    }
    Ok(words.into_iter().map(|(word, _)| word).collect())
}

/// 常用汉字区（基本区 + 扩展 A）。
fn is_han(c: char) -> bool {
    matches!(c, '\u{4e00}'..='\u{9fff}' | '\u{3400}'..='\u{4dbf}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_and_orders_words() {
        let dir = std::env::temp_dir().join("qingjian-gloss-gen-words-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("unigram.tsv");
        std::fs::write(
            &path,
            "<s>\t999\n的\t500\n开发\t300\n老司机带带我\t250\nabc\t400\n你好\t100\n",
        )
        .unwrap();
        assert_eq!(select(&path, 200, 4, None).unwrap(), ["的", "开发"]);
        assert_eq!(select(&path, 200, 4, Some(1)).unwrap(), ["的"]);
        assert!(select(&path, 1000, 4, None).is_err());
        // dict.tsv 三列格式：取最后一列，同词多读音取最大
        let dict = dir.join("dict.tsv");
        std::fs::write(
            &dict,
            "# 注释\n重庆\tchong qing\t900\n重庆\tzhong qing\t100\n开\tkai\t50\n",
        )
        .unwrap();
        assert_eq!(select(&dict, 100, 4, None).unwrap(), ["重庆"]);
    }
}
