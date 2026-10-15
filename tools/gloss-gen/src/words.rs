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
    modern_only: bool,
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
            (count >= min_count
                && (1..=max_chars).contains(&chars)
                && word.chars().all(is_han)
                && (!modern_only || is_modern_word(word)))
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

/// 越南语学习表用的现代中文过滤：不要让古汉语、方言旧称和冷僻单字污染反向越→中提示。
pub fn is_modern_word(word: &str) -> bool {
    if word.chars().any(|c| matches!(c, '\u{3400}'..='\u{4dbf}')) {
        return false;
    }
    let chars = word.chars().count();
    if chars == 1 {
        return is_common_single_char(word);
    }
    !contains_non_modern_term(word)
}

fn is_common_single_char(word: &str) -> bool {
    matches!(
        word,
        "的" | "了"
            | "是"
            | "不"
            | "我"
            | "你"
            | "他"
            | "她"
            | "它"
            | "们"
            | "在"
            | "有"
            | "和"
            | "就"
            | "都"
            | "要"
            | "想"
            | "会"
            | "能"
            | "去"
            | "来"
            | "看"
            | "说"
            | "吃"
            | "喝"
            | "买"
            | "卖"
            | "好"
            | "坏"
            | "大"
            | "小"
            | "多"
            | "少"
            | "新"
            | "老"
            | "人"
            | "家"
            | "哥"
            | "姐"
            | "弟"
            | "妹"
            | "钱"
            | "书"
            | "车"
            | "水"
            | "饭"
    )
}

fn contains_non_modern_term(word: &str) -> bool {
    NON_MODERN_TERMS.iter().any(|term| word.contains(term))
}

const NON_MODERN_TERMS: &[&str] = &[
    "吾", "伲", "俺", "偶", "鄙人", "小可", "足下", "阿拉", "堂客", "婆娘", "娘子", "妻室", "夫君",
    "夫婿", "相公", "郎君", "妾", "贱内", "拙荆", "内人", "外子", "夫人", "太太",
];

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
        assert_eq!(select(&path, 200, 4, None, false).unwrap(), ["的", "开发"]);
        assert_eq!(select(&path, 200, 4, Some(1), false).unwrap(), ["的"]);
        assert!(select(&path, 1000, 4, None, false).is_err());
        // dict.tsv 三列格式：取最后一列，同词多读音取最大
        let dict = dir.join("dict.tsv");
        std::fs::write(
            &dict,
            "# 注释\n重庆\tchong qing\t900\n重庆\tzhong qing\t100\n开\tkai\t50\n",
        )
        .unwrap();
        assert_eq!(select(&dict, 100, 4, None, false).unwrap(), ["重庆"]);
    }

    #[test]
    fn modern_filter_removes_archaic_single_chars_and_old_titles() {
        let dir = std::env::temp_dir().join("qingjian-gloss-gen-modern-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("dict.tsv");
        std::fs::write(
            &path,
            "我\two\t1000\n吾\twu\t1000\n老婆\tlao po\t900\n妻\tqi\t900\n夫君\tfu jun\t900\n朋友\tpeng you\t800\n㑇\tzhou\t800\n",
        )
        .unwrap();
        assert_eq!(
            select(&path, 1, 4, None, true).unwrap(),
            ["我", "老婆", "朋友"]
        );
    }
}
