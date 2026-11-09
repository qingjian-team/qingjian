use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::i18n::tr;

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

    /// 德语。
    German,
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
            Self::German => "de",
        }
    }
}

/// 学习语言写这个值表示不显示译文（配置 `[general] learning_language`）。
pub const LEARNING_LANGUAGE_OFF: &str = "off";

/// 设置里列的学习语言：配置写法与界面名（随界面语言），按这个顺序列出；最后一项是不显示译文。
pub fn learning_languages() -> [(&'static str, &'static str); 6] {
    [
        ("en", tr("languages.en")),
        ("ja", tr("languages.ja")),
        ("es", tr("languages.es")),
        ("vi", tr("languages.vi")),
        ("de", tr("languages.de")),
        (LEARNING_LANGUAGE_OFF, tr("languages.off")),
    ]
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
            "de" | "german" => Ok(Self::German),
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
        assert_eq!("de".parse::<Language>().unwrap(), Language::German);
        assert_eq!("DE".parse::<Language>().unwrap(), Language::German);
        assert_eq!(" de ".parse::<Language>().unwrap(), Language::German);
        assert_eq!("German".parse::<Language>().unwrap(), Language::German);
    }

    #[test]
    fn rejects_unknown_codes() {
        let error = "fr".parse::<Language>().unwrap_err();
        assert_eq!(error.0, "fr");
    }

    #[test]
    fn codes_are_iso_639_1() {
        assert_eq!(Language::Spanish.code(), "es");
        assert_eq!(Language::Japanese.code(), "ja");
        assert_eq!(Language::Vietnamese.code(), "vi");
        assert_eq!(Language::German.code(), "de");
    }

    #[test]
    fn learning_languages_parse_except_off() {
        let languages = learning_languages();
        let (off, rest) = languages.split_last().unwrap();
        assert_eq!(off.0, LEARNING_LANGUAGE_OFF);
        for (code, _) in rest {
            assert_eq!(code.parse::<Language>().unwrap().code(), *code);
        }
    }
}
