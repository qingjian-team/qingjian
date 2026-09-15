//! macOS 中英文切换配置，兼容旧版组合键写法。

use super::KeyCombo;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum ModeSwitch {
    /// 单击左或右 Shift，组合键不触发。
    #[default]
    Shift,

    /// 修饰键加字母；是否与其他快捷键冲突由 ShortcutConfig 校验。
    Combo(KeyCombo),
}

impl ModeSwitch {
    /// 配置文件中的稳定写法。
    pub fn key_string(self) -> String {
        match self {
            Self::Shift => "shift".to_owned(),
            Self::Combo(combo) => combo.key_string(),
        }
    }

    /// 设置页显示文字。
    pub fn label(self) -> String {
        match self {
            Self::Shift => "⇧（单击）".to_owned(),
            Self::Combo(combo) => combo.label(),
        }
    }

    /// 单键手势没有 KeyDown 组合键。
    pub fn combo(self) -> Option<KeyCombo> {
        match self {
            Self::Shift => None,
            Self::Combo(combo) => Some(combo),
        }
    }
}

impl FromStr for ModeSwitch {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.trim().eq_ignore_ascii_case("shift") {
            Ok(Self::Shift)
        } else {
            text.parse().map(Self::Combo)
        }
    }
}

impl TryFrom<String> for ModeSwitch {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<ModeSwitch> for String {
    fn from(value: ModeSwitch) -> Self {
        value.key_string()
    }
}
