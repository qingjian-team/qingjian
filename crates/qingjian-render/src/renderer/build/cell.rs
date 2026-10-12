//! `repeat` 展开状态条格子时每一份绑定的那一格。

use crate::renderer::StatusCell;

#[derive(Debug, Clone, Copy)]
pub(super) struct CellContext<'a> {
    pub(super) cell: &'a StatusCell,

    /// 第几格，从 0 起。
    pub(super) index: usize,

    /// 一共几格。
    pub(super) count: usize,
}
