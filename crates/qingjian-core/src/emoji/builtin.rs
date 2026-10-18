//! 编进程序的表（`crates/qingjian-core/data/`，来源见那里的 README）：补 CLDR 没有的口语 emoji 叫法、颜文字、符号，以及这些词的读音。

use std::sync::LazyLock;

use qingjian_dictionary::Dictionary;

use super::EmojiTable;

fn parse(name: &str, source: &str) -> EmojiTable {
    EmojiTable::parse_entries(source)
        .unwrap_or_else(|line| panic!("built-in {name} table line {line}: malformed entry"))
}

/// 口语叫法的 emoji（笑死 → 🤣），并在 CLDR 表后面。
pub fn emoji_supplement() -> EmojiTable {
    parse("emoji", include_str!("../../data/emoji.tsv"))
}

/// 词 → 颜文字（哭 → (T_T)）。
pub static KAOMOJI: LazyLock<EmojiTable> =
    LazyLock::new(|| parse("kaomoji", include_str!("../../data/kaomoji.tsv")));

/// 词 → 符号（约等于 → ≈，alpha → α）。
pub static SYMBOLS: LazyLock<EmojiTable> =
    LazyLock::new(|| parse("symbols", include_str!("../../data/symbols.tsv")));

/// 三张表里词库没有的词（尖括号、笑死、阿尔法），带读音与低词频，补进查词：不然整句会拼成「见括号」，配不上符号。
pub static WORDS: LazyLock<Dictionary> = LazyLock::new(|| {
    Dictionary::parse(include_str!("../../data/words.tsv"))
        .unwrap_or_else(|error| panic!("built-in words: {error}"))
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emoji::is_emoji;

    #[test]
    fn built_in_tables_parse() {
        assert!(emoji_supplement().lookup("笑死").contains(&"🤣".to_owned()));
        assert_eq!(KAOMOJI.lookup("哭")[0], "(T_T)");
        assert_eq!(SYMBOLS.lookup("alpha"), ["α", "Α"]);
        assert_eq!(SYMBOLS.lookup("约等于"), ["≈"]);
    }

    #[test]
    fn symbol_words_are_never_single_characters() {
        for word in SYMBOLS.words() {
            assert!(word.is_ascii() || word.chars().count() >= 2, "{word}");
        }
    }

    #[test]
    fn supplement_holds_only_emoji() {
        let mut table = emoji_supplement();
        let before = table.len();
        table.retain(is_emoji);
        assert_eq!(table.len(), before);
    }
}
