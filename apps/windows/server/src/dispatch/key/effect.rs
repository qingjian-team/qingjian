//! 一次按键对组句的影响。

/// 一次按键对组句的影响。
pub(crate) enum Effect {
    /// 缓冲变了，要重建 [`Composed`](super::super::Composed)；带本次要上屏的文本。
    Changed(Option<String>),

    /// 只挪了高亮 / 翻页。
    Navigated,

    /// 不吃，交还应用。
    Passthrough,
}
