//! 鼠标指针的样子：控件在 [`super::Interact::cursor`] 里声明，宿主按指针下最里层声明了的元素换系统指针。

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Cursor {
    /// 系统缺省的箭头。按钮、开关也用它（同 macOS 原生控件）。
    #[default]
    Default,

    /// 手形：链接这类点了会离开当前界面的。
    Pointer,

    /// 文字插入的 I 形：输入框。
    Text,
}
