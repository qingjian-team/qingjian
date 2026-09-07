use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// 主语言或学习语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    /// 中文，目前唯一的主语言。
    Chinese,

    /// 英语。
    English,

    /// 日语。
    Japanese,
}

impl Language {
    /// ISO 639-1 代码，用于配置与数据文件名。
    pub fn code(self) -> &'static str {
        match self {
            Self::Chinese => "zh",
            Self::English => "en",
            Self::Japanese => "ja",
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unknown language code: {0}")]
pub struct UnknownLanguage(pub String);

impl FromStr for Language {
    type Err = UnknownLanguage;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "zh" | "zh-cn" | "chinese" => Ok(Self::Chinese),
            "en" | "english" => Ok(Self::English),
            "ja" | "jp" | "japanese" => Ok(Self::Japanese),
            other => Err(UnknownLanguage(other.to_owned())),
        }
    }
}
