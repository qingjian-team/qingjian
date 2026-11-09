use serde::{Deserialize, Serialize};

use crate::i18n::tr;

/// 更新渠道。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    /// 只看正式版。
    #[default]
    Stable,

    /// 正式版与测试版（alpha / beta / rc）里最新的那个。
    Beta,
}

impl UpdateChannel {
    /// 全部取值，设置界面按这个顺序列出。
    pub const ALL: [Self; 2] = [Self::Stable, Self::Beta];

    /// 配置文件里的写法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
        }
    }

    /// 界面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Stable => tr("channel.stable"),
            Self::Beta => tr("channel.beta"),
        }
    }

    /// 索引里标着 `release_channel` 的版本在这个渠道里看不看得到。
    pub fn includes(self, release_channel: &str) -> bool {
        match self {
            Self::Stable => release_channel == "stable",
            Self::Beta => true,
        }
    }
}
