//! 上屏键的一个取值：空格、回车（都可带 Shift），或一个符号键。

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::i18n::tr;
use crate::protocol::KeyEvent;

/// Windows 的虚拟键码 `VK_SPACE`。
const VK_SPACE: u32 = 0x20;

/// Windows 的虚拟键码 `VK_RETURN`。
const VK_RETURN: u32 = 0x0D;

/// 组句中上屏用的键，配置里写成 `space`、`shift+enter`、`;` 这样。
///
/// 不收 Ctrl / Alt / Win 组合：Windows 的 TSF DLL 组句时只把不带这几个修饰键的键送进 Server
/// （Alt 组合连击键 sink 都不经过），配了也收不到。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum CommitKey {
    Space {
        shift: bool,
    },

    Enter {
        shift: bool,
    },

    /// 敲出这个符号的键；Shift 已经算在字符里（`:` 就是 Shift + `;`）。字母、数字、`'` 不行。
    Symbol(char),
}

impl CommitKey {
    /// `[shortcut] commit_candidate` 的缺省。
    pub const SPACE: Self = Self::Space { shift: false };

    /// `[shortcut] commit_raw` 的缺省。
    pub const ENTER: Self = Self::Enter { shift: false };

    /// 这次按键是不是它。只比按着的 Shift；Caps Lock 与中英模式这两个状态位不算。
    pub fn matches(self, event: &KeyEvent) -> bool {
        let modifiers = event.modifiers;
        if modifiers.has_command_key() {
            return false;
        }
        match self {
            Self::Space { shift } => event.virtual_key == VK_SPACE && modifiers.shift == shift,
            Self::Enter { shift } => event.virtual_key == VK_RETURN && modifiers.shift == shift,
            Self::Symbol(symbol) => event.character == Some(symbol),
        }
    }

    /// 配置文件里的写法。
    pub fn key_string(self) -> String {
        let named = |shift: bool, name: &str| {
            if shift {
                format!("shift+{name}")
            } else {
                name.to_owned()
            }
        };
        match self {
            Self::Space { shift } => named(shift, "space"),
            Self::Enter { shift } => named(shift, "enter"),
            Self::Symbol(symbol) => symbol.to_string(),
        }
    }

    /// 给人看的写法：`空格`、`Shift + 回车`、`;`。
    pub fn label(self) -> String {
        let named = |shift: bool, name: &str| {
            if shift {
                format!("Shift + {name}")
            } else {
                name.to_owned()
            }
        };
        match self {
            Self::Space { shift } => named(shift, tr("commit_key.space")),
            Self::Enter { shift } => named(shift, tr("commit_key.enter")),
            Self::Symbol(symbol) => symbol.to_string(),
        }
    }
}

/// 能当上屏键的符号：可见的 ASCII，不是字母、数字，也不是隔音符 `'`。
fn is_symbol(symbol: char) -> bool {
    symbol.is_ascii_graphic() && !symbol.is_ascii_alphanumeric() && symbol != '\''
}

impl FromStr for CommitKey {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let trimmed = text.trim();
        let lower = trimmed.to_ascii_lowercase();
        let (shift, name) = match lower.strip_prefix("shift+") {
            Some(name) => (true, name),
            None => (false, lower.as_str()),
        };
        match name {
            "space" => return Ok(Self::Space { shift }),
            "enter" | "return" => return Ok(Self::Enter { shift }),
            _ => {}
        }
        let mut chars = trimmed.chars();
        match (chars.next(), chars.next()) {
            (Some(symbol), None) if is_symbol(symbol) => Ok(Self::Symbol(symbol)),
            _ => Err(format!(
                "expected space, enter (optionally shift+) or one symbol key, got {text:?}"
            )),
        }
    }
}

impl TryFrom<String> for CommitKey {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<CommitKey> for String {
    fn from(key: CommitKey) -> Self {
        key.key_string()
    }
}

impl fmt::Display for CommitKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.key_string())
    }
}
