//! 下拉框弹出的菜单。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Popup {
    Font,
    Scheme,
    /// 形码方案：关、内置五笔、用户目录里的方案。
    Code,
    LearningLanguage,
    /// 输出字形：简体与三档繁体。
    Traditional,
    /// 候选后面一次最多几个 emoji。
    Emoji,
    Kaomoji,
    Symbols,
    /// 双拼键位提示什么时候弹出。
    KeyHint,
    /// 用哪一套辅码。
    AuxScheme,
    /// 设置程序与候选窗的界面语言。
    UiLanguage,
    /// 连按两次 Shift 切换到的第三输入法。
    ThirdInput,
}
