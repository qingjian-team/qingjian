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

    /// 点或拖到翻译框的原文里：光标放到第 `position` 个字之后；`extend` 时选区从原来的光标拉到这里（拖动、Shift 点击）。
    Caret { position: usize, extend: bool },

    /// 点翻译框里当前一页的第几个候选：选进框里。
    Candidate(usize),

    /// 点右上角的 ✕。
    Close,
}
