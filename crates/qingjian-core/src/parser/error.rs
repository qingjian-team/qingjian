#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ParseError {
    /// 缓冲区为空，没有可切分的内容。
    #[error("empty input")]
    Empty,

    /// 出现了拼音字母和 `'` 之外的字符。
    #[error("character {ch:?} at position {position} is not a pinyin letter")]
    InvalidChar {
        /// 从 1 开始的字符位置。
        position: usize,

        /// 违规字符。
        ch: char,
    },

    /// 没有任何一种切法能覆盖整个输入。
    #[error("cannot segment input into valid syllables")]
    NoSegmentation,
}
