use std::fmt;
use std::str::FromStr;

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use super::table::Table;
use super::{Decoded, Layout, table};
#[cfg(test)]
use crate::parser;

/// 双拼方案。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scheme {
    /// 小鹤双拼。
    Xiaohe,

    /// 自然码。
    Ziranma,

    /// 微软双拼。
    Microsoft,

    /// 搜狗双拼。
    Sogou,

    /// 智能 ABC。
    Abc,

    /// 小浪双拼。
    Xiaolang,

    /// 首道双拼。
    Shoudao,

    /// 拼音加加双拼。
    Jiajia,

    /// 紫光双拼。
    Ziguang,
}

impl Scheme {
    /// 全部方案，设置界面按这个顺序列出。
    pub const ALL: [Self; 9] = [
        Self::Xiaohe,
        Self::Ziranma,
        Self::Microsoft,
        Self::Sogou,
        Self::Abc,
        Self::Xiaolang,
        Self::Shoudao,
        Self::Jiajia,
        Self::Ziguang,
    ];

    /// 配置文件里的写法。`const`：设置界面要按方案列表建常量表（见 `qingjian_platform::Scheme`）。
    pub const fn key(self) -> &'static str {
        match self {
            Self::Xiaohe => "xiaohe",
            Self::Ziranma => "ziranma",
            Self::Microsoft => "microsoft",
            Self::Sogou => "sogou",
            Self::Abc => "abc",
            Self::Xiaolang => "xiaolang",
            Self::Shoudao => "shoudao",
            Self::Jiajia => "jiajia",
            Self::Ziguang => "ziguang",
        }
    }

    /// 界面上的名字。`const` 的理由同上。
    pub const fn label(self) -> &'static str {
        match self {
            Self::Xiaohe => "小鹤双拼",
            Self::Ziranma => "自然码",
            Self::Microsoft => "微软双拼",
            Self::Sogou => "搜狗双拼",
            Self::Abc => "智能ABC",
            Self::Xiaolang => "小浪双拼",
            Self::Shoudao => "首道双拼",
            Self::Jiajia => "拼音加加",
            Self::Ziguang => "紫光双拼",
        }
    }

    fn table(self) -> &'static Table {
        match self {
            Self::Xiaohe => &table::XIAOHE,
            Self::Ziranma => &table::ZIRANMA,
            Self::Microsoft => &table::MICROSOFT,
            Self::Sogou => &table::SOGOU,
            Self::Abc => &table::ABC,
            Self::Xiaolang => &table::XIAOLANG,
            Self::Shoudao => &table::SHOUDAO,
            Self::Jiajia => &table::JIAJIA,
            Self::Ziguang => &table::ZIGUANG,
        }
    }

    /// 这套方案的键位。
    pub fn layout(self) -> &'static Layout {
        let index = Self::ALL.iter().position(|scheme| *scheme == self);
        &BUILTIN[index.unwrap_or_default()]
    }

    /// 这套方案是否用到 `;` 键。
    pub fn uses_semicolon(self) -> bool {
        self.layout().uses_semicolon()
    }

    /// 见 [`Layout::is_key`]。
    pub fn is_key(self, c: char) -> bool {
        self.layout().is_key(c)
    }

    /// 见 [`Layout::initial`]。
    pub fn initial(self, key: char) -> Option<&'static str> {
        self.layout().initial(key)
    }

    /// 见 [`Layout::finals`]。
    pub fn finals(self, key: char) -> &'static [String] {
        self.layout().finals(key)
    }

    /// 见 [`Layout::syllable`]。
    pub fn syllable(self, first: char, second: char) -> Option<String> {
        self.layout().syllable(first, second)
    }

    /// 见 [`Layout::encode`]。
    pub fn encode(self, syllable: &str) -> Option<[char; 2]> {
        self.layout().encode(syllable)
    }

    /// 见 [`Layout::decode`]。
    pub fn decode(self, keys: &str) -> Decoded {
        self.layout().decode(keys)
    }

    /// 见 [`Layout::decode_mixed`]。
    pub fn decode_mixed(self, keys: &str, fuzzy: crate::fuzzy::FuzzyRules) -> Vec<Decoded> {
        self.layout().decode_mixed(keys, fuzzy)
    }
}

/// 内置九套的键位，与 [`Scheme::ALL`] 同序。
static BUILTIN: LazyLock<[Layout; 9]> =
    LazyLock::new(|| Scheme::ALL.map(|scheme| Layout::from_table(scheme.key(), scheme.table())));

impl FromStr for Scheme {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|scheme| scheme.key() == text.trim().to_ascii_lowercase())
            .ok_or_else(|| format!("unknown shuangpin scheme: {text:?}"))
    }
}

impl fmt::Display for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.key())
    }
}

#[cfg(test)]
mod tests;
