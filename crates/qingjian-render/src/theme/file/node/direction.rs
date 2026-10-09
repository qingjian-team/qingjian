//! 框里子节点的排列方向。

use serde::Deserialize;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Direction {
    /// 横向。
    #[default]
    Row,

    /// 纵向。
    Column,
}
