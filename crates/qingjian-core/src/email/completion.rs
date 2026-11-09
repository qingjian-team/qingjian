//! 一条邮箱后缀补全。

/// 候选上显示 [`Self::domain`]，选中上屏 [`Self::insert`]（去掉已经敲了的那截）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailCompletion {
    pub domain: &'static str,

    pub insert: String,
}
