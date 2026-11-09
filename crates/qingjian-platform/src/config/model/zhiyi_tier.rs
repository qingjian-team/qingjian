use qingjian_core::ChoiceTier;
use serde::{Deserialize, Serialize};

use crate::i18n::tr;

/// 知意的档位：比的候选越多、看的前文越长，挑得越准，也越费电。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ZhiyiTier {
    Saver,
    #[default]
    Balanced,
    Accurate,
}

impl ZhiyiTier {
    /// 全部取值，设置界面按这个顺序列出。
    pub const ALL: [Self; 3] = [Self::Saver, Self::Balanced, Self::Accurate];

    /// 配置文件里的写法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Saver => "saver",
            Self::Balanced => "balanced",
            Self::Accurate => "accurate",
        }
    }

    /// 界面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Saver => tr("zhiyi_tier.saver"),
            Self::Balanced => tr("zhiyi_tier.balanced"),
            Self::Accurate => tr("zhiyi_tier.accurate"),
        }
    }

    pub fn choice_tier(self) -> ChoiceTier {
        match self {
            Self::Saver => ChoiceTier::Saver,
            Self::Balanced => ChoiceTier::Balanced,
            Self::Accurate => ChoiceTier::Accurate,
        }
    }
}
