//! BIP-39 英文助记词表：钱包的 12 / 24 词助记词全部从这 2048 个常见英文单词里选。
//!
//! 用途两处：一是拦掉 [`super::learning::MutedLearner::learn_english`] 里的持久化——命中表内的词
//! 不该被当成「新学到的英文词」写进 `user-english.tsv`；二是喂给 [`super::Engine`] 里的连续命中计数，
//! 连续敲中够多个表内词时临时转成私密输入（见 [`super::Engine::note_english_commit`]），
//! 不依赖密码框识别——很多钱包插件的助记词导入框用的是普通文本框，不会触发那层判定。
//!
//! 词表本身都是日常英文常用词（`about`、`age`、`all` 这类），单独出现在正常英文写作里很常见，
//! 所以只做逐词黑名单还不够——真正的信号是「连续一串都命中」，见 [`super::BIP39_STREAK_THRESHOLD`]。
//!
//! 词表来源：BIP-39 官方英文词表（2048 词，按字母排好序），
//! <https://github.com/bitcoin/bips/blob/master/bip-0039/english.txt>。

use std::collections::HashSet;
use std::sync::LazyLock;

static WORDLIST: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    include_str!("bip39_english.txt")
        .lines()
        .filter(|line| !line.is_empty())
        .collect()
});

/// `word` 是不是 BIP-39 英文词表里的词，忽略大小写。词表本身全部是小写，
/// 已经是小写的输入（绝大多数情况）不额外分配。
pub(super) fn is_bip39_word(word: &str) -> bool {
    if word.bytes().all(|b| !b.is_ascii_uppercase()) {
        WORDLIST.contains(word)
    } else {
        WORDLIST.contains(word.to_ascii_lowercase().as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wordlist_has_2048_entries_and_no_duplicates() {
        let raw: Vec<&str> = include_str!("bip39_english.txt")
            .lines()
            .filter(|line| !line.is_empty())
            .collect();
        assert_eq!(raw.len(), 2048);
        assert_eq!(WORDLIST.len(), 2048, "词表里不该有重复词");
    }

    #[test]
    fn matches_case_insensitively() {
        assert!(is_bip39_word("abandon"));
        assert!(is_bip39_word("Abandon"));
        assert!(is_bip39_word("ABANDON"));
        assert!(is_bip39_word("zoo"));
    }

    #[test]
    fn rejects_words_outside_the_list() {
        assert!(!is_bip39_word(""));
        assert!(!is_bip39_word("qingjian"));
        assert!(!is_bip39_word("rust"));
        // 前缀命中不算：词表要求整词精确匹配
        assert!(!is_bip39_word("aban"));
    }
}
