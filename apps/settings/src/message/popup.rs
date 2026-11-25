//! 下拉框与「…」按钮弹出的菜单。

use crate::library::Kind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Popup {
    Font,
    Scheme,
    /// 形码方案：关、内置五笔、用户目录里的方案。
    Code,
    AutoPhraseLength,
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
    /// 导入的词库、码表那一行的「…」：第几本（[`crate::library::Library::imported`] 里的序号）的导出、移除。
    Entry(Kind, usize),
    /// 照 Windows 设置画时几选一的设置行用下拉框（不用分段），按它的本地名区分；菜单建行时当场建。
    Choice(&'static str),
}
