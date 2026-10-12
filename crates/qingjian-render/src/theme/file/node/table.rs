//! 表格：`repeat` 出来的每个组件是一行，组件的子节点依次是各列；每列取各行最宽，行高固定。

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub(crate) struct TableSpec {
    /// 行高（点）。
    pub(crate) row_height: f32,
}
