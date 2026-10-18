//! 一个元素当前的交互状态，建树时由 [`super::Ui::state`] 取，控件按它挑样式（设计稿里组件的 State 变体）。

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ElementState {
    /// 指针在它或它的子元素上。
    pub hovered: bool,

    /// 按下了它、指针还在它上面；拖出去就不算，拖回来又算（同原生按钮）。
    pub pressed: bool,

    pub focused: bool,

    /// 有焦点且是键盘操作带来的：只有这时画焦点环，鼠标点出来的焦点不画。
    pub focus_ring: bool,
}
