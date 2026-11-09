//! [`Hotkey`]：呼出面板的组合键及其配置写法。

/// 呼出面板的组合键。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hotkey {
    pub control: bool,

    pub shift: bool,

    pub alt: bool,

    /// Windows 键。
    pub win: bool,

    /// 字母、数字或句号（小写）。
    pub key: char,
}

impl Hotkey {
    /// 与输入法配置的缺省一致：Windows 是 Ctrl + Alt + V，macOS 是 ⌘⇧V。
    #[cfg(not(target_os = "macos"))]
    pub const DEFAULT: Self = Self {
        control: true,
        shift: false,
        alt: true,
        win: false,
        key: 'v',
    };

    #[cfg(target_os = "macos")]
    pub const DEFAULT: Self = Self {
        control: false,
        shift: true,
        alt: false,
        win: true,
        key: 'v',
    };

    /// 配置的写法：`ctrl+alt+v`，修饰键也认 `control`、`option`、`cmd`、`win`，顺序随意；至少一个修饰键。
    pub fn parse(text: &str) -> Option<Self> {
        let (modifiers, key) = text.trim().rsplit_once('+')?;
        let mut chars = key.trim().chars();
        let (Some(key), None) = (chars.next(), chars.next()) else {
            return None;
        };
        if !key.is_ascii_alphanumeric() && key != '.' {
            return None;
        }
        let mut hotkey = Self {
            control: false,
            shift: false,
            alt: false,
            win: false,
            key: key.to_ascii_lowercase(),
        };
        for token in modifiers
            .split(['+', ' '])
            .filter(|token| !token.is_empty())
        {
            match token.to_ascii_lowercase().as_str() {
                "option" | "alt" | "⌥" => hotkey.alt = true,
                "shift" | "⇧" => hotkey.shift = true,
                "control" | "ctrl" | "⌃" => hotkey.control = true,
                "command" | "cmd" | "super" | "win" | "⌘" => hotkey.win = true,
                _ => return None,
            }
        }
        (hotkey.control || hotkey.shift || hotkey.alt || hotkey.win).then_some(hotkey)
    }
}
