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

#[cfg(test)]
mod tests {
    use super::Hotkey;

    #[test]
    fn parse_reads_ctrl_alt_v() {
        let hotkey = Hotkey::parse("ctrl+alt+v").unwrap();
        assert!(hotkey.control);
        assert!(!hotkey.shift);
        assert!(hotkey.alt);
        assert!(!hotkey.win);
        assert_eq!(hotkey.key, 'v');
    }

    #[test]
    fn parse_is_case_insensitive_and_lowercases_the_key() {
        let hotkey = Hotkey::parse("CTRL+ALT+V").unwrap();
        assert_eq!(hotkey.key, 'v');
        assert!(hotkey.control && hotkey.alt);
    }

    #[test]
    fn parse_accepts_every_alias() {
        for text in ["control+shift+v", "ctrl+shift+v", "⌃+⇧+v"] {
            let hotkey = Hotkey::parse(text).unwrap();
            assert!(hotkey.control && hotkey.shift, "{text}");
        }
        for text in ["command+v", "cmd+v", "super+v", "win+v", "⌘+v"] {
            let hotkey = Hotkey::parse(text).unwrap();
            assert!(hotkey.win && !hotkey.control, "{text}");
        }
        for text in ["option+v", "alt+v", "⌥+v"] {
            let hotkey = Hotkey::parse(text).unwrap();
            assert!(hotkey.alt && !hotkey.control, "{text}");
        }
    }

    #[test]
    fn parse_ignores_spaces_around_modifiers() {
        assert_eq!(Hotkey::parse("ctrl + alt + v"), Hotkey::parse("ctrl+alt+v"));
        assert_eq!(Hotkey::parse("ctrl+alt+ v"), Hotkey::parse("ctrl+alt+v"));
    }

    #[test]
    fn parse_accepts_digits_and_period() {
        assert_eq!(Hotkey::parse("ctrl+1").unwrap().key, '1');
        assert_eq!(Hotkey::parse("ctrl+alt+.").unwrap().key, '.');
    }

    #[test]
    fn parse_requires_a_single_key() {
        assert!(Hotkey::parse("ctrl+vv").is_none());
        assert!(Hotkey::parse("ctrl+").is_none());
        assert!(Hotkey::parse("ctrl+alt").is_none());
    }

    #[test]
    fn parse_rejects_punctuation_other_than_period() {
        assert!(Hotkey::parse("ctrl+alt+;").is_none());
        assert!(Hotkey::parse("ctrl+alt+空格").is_none());
    }

    #[test]
    fn parse_requires_at_least_one_known_modifier() {
        assert!(Hotkey::parse("v").is_none());
        assert!(Hotkey::parse("hyper+v").is_none());
        assert!(Hotkey::parse("+v").is_none());
    }

    #[test]
    fn default_matches_the_platform() {
        let default = Hotkey::DEFAULT;
        assert_eq!(default.key, 'v');
        #[cfg(not(target_os = "macos"))]
        {
            assert!(default.control && default.alt && !default.shift && !default.win);
        }
        #[cfg(target_os = "macos")]
        {
            assert!(default.win && default.shift && !default.control && !default.alt);
        }
    }
}
