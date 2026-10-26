//! 双拼方案的 `scheme.toml`：名字、拿哪套内置方案当底子，以及要改的键。

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::SchemeMeta;

/// `scheme.toml` 的内容。三张表都按键（零声母按音节）整条替换底子里的那一条，值写空就是撤掉。
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub(super) struct Descriptor {
    /// 必须是 [`super::KIND`]，形码方案与双拼方案共用 `schemes/` 目录，靠它区分。
    #[serde(rename = "type")]
    pub kind: String,

    /// 界面与状态条上的名字；空着时用目录名。
    pub name: String,

    #[serde(flatten)]
    pub meta: SchemeMeta,

    /// 拿哪套内置方案当底子（`xiaohe` 这类写法）；空着就从空白开始，每个键都要自己写。
    pub base: String,

    /// 翘舌声母键：键 → `zh` / `ch` / `sh`。
    pub initials: BTreeMap<String, String>,

    /// 韵母键：键 → 一个韵母或按优先级排的几个。
    pub finals: BTreeMap<String, OneOrMany>,

    /// 零声母：音节 → 一种写法或几种（第一种是主写法）。
    pub zero: BTreeMap<String, OneOrMany>,
}

/// 写一个可以不加方括号。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub(super) enum OneOrMany {
    One(String),
    Many(Vec<String>),
}

impl OneOrMany {
    pub fn into_vec(self) -> Vec<String> {
        match self {
            Self::One(one) if one.trim().is_empty() => Vec::new(),
            Self::One(one) => vec![one.trim().to_owned()],
            Self::Many(many) => many.into_iter().map(|s| s.trim().to_owned()).collect(),
        }
    }
}
