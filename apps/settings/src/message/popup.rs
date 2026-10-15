//! 下拉框弹出的菜单。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Popup {
    Font,
    Scheme,
    LearningLanguage,
    /// 输出字形：简体与三档繁体。
    Traditional,
    /// 候选后面一次最多几个 emoji。
    Emoji,
    Kaomoji,
    Symbols,
}
