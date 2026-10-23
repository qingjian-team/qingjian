//! 横排矩阵的状态（「横排时展开成多行」开没开、展没展开），决定 ↓ 能不能展开、↑ 是换行还是回拼音行。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Matrix {
    /// 没开「横排时展开成多行」。
    Off,

    /// 开着，还是单行。
    Closed,

    /// 已展开；`first_row` 是高亮在不在整个矩阵的第一行（视口可能已往下滚）。
    Open { first_row: bool },
}
