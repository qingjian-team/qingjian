//! 越南语输入：Simple Telex 组词、越英混输与原始按键恢复。

mod telex;

pub use telex::TelexComposer;

use qingjian_dictionary::WordList;

/// 当前 Telex 结果的词库补全；精确结果由调用方放第一，这里只返回更长的词。
pub fn complete(words: &WordList, prefix: &str, limit: usize) -> Vec<String> {
    if prefix.chars().count() < 2 {
        return Vec::new();
    }
    words
        .complete(&prefix.to_lowercase(), limit)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

/// 连续输入候选：完整短语前缀优先，不足时再用当前词前缀补齐。
pub fn suggest(words: &WordList, phrase_prefix: &str, current: &str, limit: usize) -> Vec<String> {
    if limit == 0 {
        return Vec::new();
    }
    let mut result = complete(words, phrase_prefix, limit);
    if result.len() < limit && phrase_prefix != current {
        for word in complete(words, current, limit - result.len()) {
            if !result.contains(&word) {
                result.push(word);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use qingjian_dictionary::WordList;

    use super::{complete, suggest};

    #[test]
    fn completes_vietnamese_prefixes_by_frequency() {
        let words = WordList::parse(
            "tiếng Việt\ttiếng việt\t20\ntiếng Anh\ttiếng anh\t30\ntiếng\ttiếng\t100\ntim\ttim\t90\n",
        )
        .unwrap();
        assert!(complete(&words, "t", 9).is_empty());
        assert_eq!(complete(&words, "tiếng", 9), ["tiếng Anh", "tiếng Việt"]);
    }

    #[test]
    fn continuous_phrase_matches_before_current_word_fallbacks() {
        let words = WordList::parse(
            "tôi là ai\ttôi là ai\t20\ntôi làm việc\ttôi làm việc\t10\nlàm việc\tlàm việc\t90\nlàng\tlàng\t80\n",
        )
        .unwrap();
        assert_eq!(
            suggest(&words, "tôi là", "là", 4),
            ["tôi là ai", "tôi làm việc", "làm việc", "làng"]
        );
    }
}
