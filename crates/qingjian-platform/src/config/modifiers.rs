use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// 一组修饰键。配置里写成 `shift+option` 这样的串（顺序随意，`alt` / `ctrl` / `cmd` 也认）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Modifiers {
    /// ⌥
    pub option: bool,

    /// ⇧
    pub shift: bool,

    /// ⌃
    pub control: bool,

    /// ⌘
    pub command: bool,
}

impl Modifiers {
    pub const OPTION: Self = Self {
        option: true,
        shift: false,
        control: false,
        command: false,
    };

    pub const SHIFT_OPTION: Self = Self {
        option: true,
        shift: true,
        control: false,
        command: false,
    };

    pub const SHIFT: Self = Self {
        option: false,
        shift: true,
        control: false,
        command: false,
    };

    /// ⌃ / Ctrl。Windows 上译词键的缺省（Alt 会被系统菜单截走）。
    pub const CONTROL: Self = Self {
        option: false,
        shift: false,
        control: true,
        command: false,
    };

    /// ⇧⌃ / Shift+Ctrl。
    pub const SHIFT_CONTROL: Self = Self {
        option: false,
        shift: true,
        control: true,
        command: false,
    };

    pub fn is_empty(&self) -> bool {
        !(self.option || self.shift || self.control || self.command)
    }

    /// 配置文件里的写法，固定顺序 control+shift+option+command。
    pub fn key(&self) -> String {
        let mut parts = Vec::new();
        if self.control {
            parts.push("control");
        }
        if self.shift {
            parts.push("shift");
        }
        if self.option {
            parts.push("option");
        }
        if self.command {
            parts.push("command");
        }
        parts.join("+")
    }

    /// 给人看的符号，按 macOS 的习惯顺序 ⌃⇧⌥⌘。
    pub fn label(&self) -> String {
        let mut out = String::new();
        if self.control {
            out.push('⌃');
        }
        if self.shift {
            out.push('⇧');
        }
        if self.option {
            out.push('⌥');
        }
        if self.command {
            out.push('⌘');
        }
        out
    }
}

impl FromStr for Modifiers {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut out = Self::default();
        for token in text.split(['+', ' ']).filter(|t| !t.is_empty()) {
            match token.to_ascii_lowercase().as_str() {
                "option" | "alt" | "⌥" => out.option = true,
                "shift" | "⇧" => out.shift = true,
                "control" | "ctrl" | "⌃" => out.control = true,
                "command" | "cmd" | "⌘" => out.command = true,
                other => return Err(format!("unknown modifier: {other}")),
            }
        }
        if out.is_empty() {
            return Err("no modifier given".to_owned());
        }
        Ok(out)
    }
}

impl TryFrom<String> for Modifiers {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<Modifiers> for String {
    fn from(modifiers: Modifiers) -> Self {
        modifiers.key()
    }
}

impl fmt::Display for Modifiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.key())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_aliases_in_any_order_and_prints_canonically() {
        assert_eq!("option".parse::<Modifiers>().unwrap(), Modifiers::OPTION);
        assert_eq!(
            "alt+shift".parse::<Modifiers>().unwrap(),
            Modifiers::SHIFT_OPTION
        );
        assert_eq!(Modifiers::SHIFT_OPTION.key(), "shift+option");
        assert_eq!(Modifiers::SHIFT_OPTION.label(), "⇧⌥");
        assert!("".parse::<Modifiers>().is_err());
        assert!("hyper".parse::<Modifiers>().is_err());
        for option in [
            "option",
            "shift+option",
            "control+option",
            "control+shift+command",
        ] {
            assert_eq!(option.parse::<Modifiers>().unwrap().key(), option);
        }
    }
}
