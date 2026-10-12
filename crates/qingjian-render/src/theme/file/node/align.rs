//! 节点在父框交叉轴上的对齐。

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Align {
    Start,

    End,

    Center,

    /// 撑满。
    Stretch,
}
