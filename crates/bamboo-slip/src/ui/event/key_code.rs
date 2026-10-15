//! 交互层认得的按键；其余的按键带着字符交给元素自己的按键处理。

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyCode {
    Tab,
    Enter,
    Space,
    Escape,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Home,
    End,
    Backspace,
    Delete,

    /// 产生字符的键（已按修饰键与键盘布局算好）。
    Character(String),
}
