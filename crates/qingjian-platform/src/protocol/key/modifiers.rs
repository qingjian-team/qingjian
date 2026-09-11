use serde::{Deserialize, Serialize};

/// 一次按键按下时的修饰键状态（Windows 语义）。
///
/// 普通字符键通常四个都是 `false`，能干净序列化——不像配置层的 `Modifiers`（走字符串、空值不合法，
/// 那个是给快捷键配置用的）。协议要能表达「没有修饰键」，所以自带这个而不复用它。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct KeyModifiers {
    /// Ctrl
    pub ctrl: bool,

    /// Shift
    pub shift: bool,

    /// Alt
    pub alt: bool,

    /// Win（⊞）
    pub win: bool,
}
