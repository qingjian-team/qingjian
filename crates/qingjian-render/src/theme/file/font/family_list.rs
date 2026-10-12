//! 字族回退链：`"Songti SC"` 或 `["Songti SC", "STSong", "system"]`，按顺序用第一个装了的；`system` 是界面字体（用户在设置里选的字体，没选就是系统字体）。

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(from = "Raw")]
pub(crate) struct FamilyList(pub(crate) Vec<String>);

#[derive(Deserialize)]
#[serde(untagged)]
enum Raw {
    One(String),

    Many(Vec<String>),
}

impl From<Raw> for FamilyList {
    fn from(raw: Raw) -> Self {
        Self(match raw {
            Raw::One(name) => vec![name],
            Raw::Many(names) => names,
        })
    }
}

impl FamilyList {
    /// 链里代表界面字体的名字。
    pub(crate) const SYSTEM: &str = "system";

    /// 只有界面字体。
    pub(crate) fn system() -> Self {
        Self(vec![Self::SYSTEM.to_owned()])
    }

    /// 是不是界面字体这个名字。
    pub(crate) fn is_system(name: &str) -> bool {
        name.eq_ignore_ascii_case(Self::SYSTEM)
    }
}
