//! 一次「相遇」：用户上屏了一段中文，它有学习语言的对应说法。生词本的时间线一条就是一次。

use crate::candidate::Language;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Encounter {
    pub language: Language,

    /// 上屏的中文（词或整句）。
    pub text: String,

    /// 学习语言的说法：词是译词表的第一条，整句是云端翻的。
    pub translation: String,

    /// 读音：日语的假名；英语等以后有音标数据再填。
    pub reading: Option<String>,
}
