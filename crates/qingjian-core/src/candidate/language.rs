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

    /// 西班牙语。
    Spanish,

    /// 越南语。
    Vietnamese,
}

impl Language {
    /// ISO 639-1 代码，用于配置与数据文件名。
    pub fn code(self) -> &'static str {
        match self {
            Self::Chinese => "zh",
            Self::English => "en",
            Self::Japanese => "ja",
            Self::Spanish => "es",
            Self::Vietnamese => "vi",
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
            "es" | "es-es" | "spanish" => Ok(Self::Spanish),
            "vi" | "vi-vn" | "vietnamese" => Ok(Self::Vietnamese),
            other => Err(UnknownLanguage(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_language_codes_case_insensitively() {
        assert_eq!("es".parse::<Language>().unwrap(), Language::Spanish);
        assert_eq!("ES".parse::<Language>().unwrap(), Language::Spanish);
        assert_eq!(" es-ES ".parse::<Language>().unwrap(), Language::Spanish);
        assert_eq!("Spanish".parse::<Language>().unwrap(), Language::Spanish);
        assert_eq!("vi-VN".parse::<Language>().unwrap(), Language::Vietnamese);
        assert_eq!(
            "Vietnamese".parse::<Language>().unwrap(),
            Language::Vietnamese
        );
        assert_eq!("zh-cn".parse::<Language>().unwrap(), Language::Chinese);
    }

    #[test]
    fn rejects_unknown_codes() {
        let error = "de".parse::<Language>().unwrap_err();
        assert_eq!(error.0, "de");
    }

    #[test]
    fn codes_are_iso_639_1() {
        assert_eq!(Language::Spanish.code(), "es");
        assert_eq!(Language::Japanese.code(), "ja");
        assert_eq!(Language::Vietnamese.code(), "vi");
    }
}
