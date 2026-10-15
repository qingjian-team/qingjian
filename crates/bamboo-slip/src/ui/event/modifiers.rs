//! 按键时按住的修饰键。`command` 是 macOS 的 ⌘、其他平台的 Ctrl，快捷键按它判断；
//! `control`、`logo` 是键盘上的原样（⌃ / Ctrl、⌘ / Win），录快捷键时用。

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,

    pub alt: bool,

    pub command: bool,

    /// ⌃（macOS）/ Ctrl。
    pub control: bool,

    /// ⌘（macOS）/ Win / Super。
    pub logo: bool,
}
