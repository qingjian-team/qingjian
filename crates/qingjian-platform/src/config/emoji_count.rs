//! `[general] emoji` / `kaomoji` / `symbols`：候选后面一次最多配几个，0 不配。旧配置的 emoji 写的 `true` / `false` 读成 3 / 0，
//! 认不出的写法按缺省读，不让这一项把整份配置拖成解析失败。

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct EmojiCount(u8);

impl EmojiCount {
    /// 上限与 Core 一致：再多 emoji 就把字词挤到第二页了。
    pub const MAX: u8 = 3;

    /// 设置里下拉的选项，下标就是个数。
    pub const LABELS: [&str; Self::MAX as usize + 1] =
        ["不显示", "最多 1 个", "最多 2 个", "最多 3 个"];

    pub fn new(count: u8) -> Self {
        Self(count.min(Self::MAX))
    }

    pub fn get(self) -> u8 {
        self.0
    }
}

impl Default for EmojiCount {
    fn default() -> Self {
        Self(Self::MAX)
    }
}

impl<'de> Deserialize<'de> for EmojiCount {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Flag(bool),
            Count(i64),
            Other(serde::de::IgnoredAny),
        }
        Ok(match Raw::deserialize(deserializer)? {
            Raw::Flag(true) => Self::default(),
            Raw::Flag(false) => Self(0),
            Raw::Count(count) => Self(count.clamp(0, i64::from(Self::MAX)) as u8),
            Raw::Other(_) => Self::default(),
        })
    }
}
