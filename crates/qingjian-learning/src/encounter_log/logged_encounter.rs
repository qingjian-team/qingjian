//! 时间线上的一条：什么时候、哪门语言、上屏的中文、对应的说法与读音。

use jiff::civil::DateTime;
use qingjian_core::Language;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoggedEncounter {
    /// 本机时间，到秒。
    pub time: DateTime,

    pub language: Language,

    /// 上屏的中文。
    pub text: String,

    /// 学习语言的说法。
    pub translation: String,

    /// 读音（日语假名、以后的音标），没有为空。
    pub reading: String,
}
