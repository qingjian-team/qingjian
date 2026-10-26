//! 一条算式结果补全。

/// 候选上显示 [`Self::label`]；选中时先删掉光标前 [`Self::replace`] 个字符，再上屏 [`Self::insert`]。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalculationCompletion {
    pub label: String,

    pub insert: String,

    /// 上屏前删掉光标前几个字符（都是 ASCII 或 × ÷，按字符数与 UTF-16 单位数相同）。
    pub replace: usize,
}
