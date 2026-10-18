//! 鼠标在符号面板里做了什么。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelMouse {
    /// 点格子：视口里第几格（行优先），`double` 是双击。
    Cell { offset: usize, double: bool },

    /// 点分组胶囊，第几个。
    Group(usize),

    /// 点当前组的分类标签，第几个。
    Section(usize),

    /// 滚轮 / 触控板滚了几行，正数往下。
    Scroll(isize),

    /// 点右上角的 ✕。
    Close,
}
