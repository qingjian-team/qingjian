use std::ops::{Add, AddAssign};

use crate::sentence::is_han;

/// 一次上屏折成的用量；加起来就是一段时间的用量。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    /// 汉字数：上屏文字里的汉字，标点、字母、emoji 不算。
    pub hanzi: u64,

    /// 中文词数：选一个词算一个，整句按切出来的词数算，译词 / 英文 / 快捷候选不算。
    pub words: u64,

    /// 英文词数：英文候选、原样上屏的英文词、英文译词按空格分开数。
    pub english_words: u64,

    /// 上屏次数。
    pub commits: u64,

    /// 从候选里选的上屏（原样上屏的字母串不算）；下面几项只在这些上屏上数。
    pub choices: u64,

    /// 选的是第一个候选。
    pub first_choices: u64,

    /// 选词上屏里有汉字的那些，汉字共几个：速度与码长的分子，与下面两项同一批上屏，不含老记录与英文候选。
    pub typed_hanzi: u64,

    /// 敲进组句的键数（字母、数字、分隔符），退格不算；只数有汉字的选词上屏。
    pub keys: u64,

    /// 实际打字的毫秒数：从上一次上屏（或输入法刚用上）到这次上屏，键与键之间停顿太久的那段不算；只数有汉字的选词上屏。
    pub typing_ms: u64,

    /// 上屏前退格或删过的。
    pub edited: u64,
}

impl Usage {
    /// 按上屏的文字本身数汉字与英文词，算一次上屏；中文词数由调用方按来源定（见 `Engine`）。
    pub fn of_text(text: &str) -> Self {
        Self {
            hanzi: text.chars().filter(|c| is_han(*c)).count() as u64,
            words: 0,
            english_words: count_english_words(text),
            commits: 1,
            ..Self::default()
        }
    }

    /// 一次都没记过。
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

impl AddAssign for Usage {
    fn add_assign(&mut self, other: Self) {
        self.hanzi += other.hanzi;
        self.words += other.words;
        self.english_words += other.english_words;
        self.commits += other.commits;
        self.choices += other.choices;
        self.first_choices += other.first_choices;
        self.typed_hanzi += other.typed_hanzi;
        self.keys += other.keys;
        self.typing_ms += other.typing_ms;
        self.edited += other.edited;
    }
}

impl Add for Usage {
    type Output = Self;

    fn add(mut self, other: Self) -> Self {
        self += other;
        self
    }
}

/// 拉丁字母组成的词有几个：`don't`、`open-source` 各算一个，纯数字不算。
fn count_english_words(text: &str) -> u64 {
    text.split(|c: char| !(c.is_ascii_alphabetic() || c == '\'' || c == '-'))
        .filter(|token| token.chars().any(|c| c.is_ascii_alphabetic()))
        .count() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_hanzi_and_english_words_separately() {
        let usage = Usage::of_text("你好，world！");
        assert_eq!(
            usage,
            Usage {
                hanzi: 2,
                words: 0,
                english_words: 1,
                commits: 1,
                ..Usage::default()
            }
        );
        assert_eq!(Usage::of_text("open-source don't 2024").english_words, 2);
        assert_eq!(Usage::of_text("😀").hanzi, 0);
    }

    #[test]
    fn adds_up() {
        let mut total = Usage::default();
        assert!(total.is_empty());
        total += Usage::of_text("你好");
        total += Usage::of_text("world");
        assert_eq!(total.hanzi, 2);
        assert_eq!(total.english_words, 1);
        assert_eq!(total.commits, 2);
        assert!(!total.is_empty());
    }
}
