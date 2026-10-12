//! `repeat` 展开时每一份绑定的那一项候选。

use crate::frame::Row;

#[derive(Debug, Clone, Copy)]
pub(super) struct RowContext<'a> {
    pub(super) row: &'a Row,

    /// 在这一页里的位置，从 0 起。
    pub(super) index: usize,

    /// 这一页的候选数。
    pub(super) count: usize,
}
