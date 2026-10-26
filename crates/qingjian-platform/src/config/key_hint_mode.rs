//! 双拼键位提示什么时候弹出。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyHintMode {
    #[default]
    Off,

    /// 敲到一半停下来、或敲出拼不成音节的键时才弹出。
    Auto,

    /// 组句时一直显示。
    Always,
}

impl KeyHintMode {
    /// 全部取值，设置界面按这个顺序列出。
    pub const ALL: [Self; 3] = [Self::Off, Self::Auto, Self::Always];

    /// 配置文件里的写法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Auto => "auto",
            Self::Always => "always",
        }
    }

    /// 界面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "关",
            Self::Auto => "卡住时显示",
            Self::Always => "一直显示",
        }
    }
}
