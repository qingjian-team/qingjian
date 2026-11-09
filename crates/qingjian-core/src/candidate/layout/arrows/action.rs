//! 一个方向键要壳做的事。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowAction {
    /// 拼音光标左（-1）右（1）移一格。
    MoveCursor(isize),

    /// 高亮在候选之间按阅读顺序移一个，越过页边翻页。
    MoveHighlight(isize),

    /// 矩阵里上（-1）下（1）换一行。
    MoveRow(isize),

    /// 单行展开成矩阵。
    Expand,

    /// 换到另一区，光标与高亮都不动。
    Focus(super::Focus),

    /// 没有去处，键照样吃掉。
    Stay,
}
