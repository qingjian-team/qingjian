//! 一段文字量出来的尺寸（像素）。

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TextSize {
    pub width: f32,

    /// 等于所用样式的行高。
    pub height: f32,

    /// 第一行的基线离行框顶边多远，与画字时一样取整。
    pub baseline: f32,
}
