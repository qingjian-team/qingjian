//! 候选窗口两种排布各自的根节点。

use serde::Deserialize;

use super::node::NodeSpec;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Windows {
    /// 竖排：一行一个候选。
    pub(crate) vertical: NodeSpec,

    /// 横排：候选排成一行。
    pub(crate) horizontal: NodeSpec,
}
