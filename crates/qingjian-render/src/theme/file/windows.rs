//! 候选窗口各种排布的根节点。

use serde::Deserialize;

use super::node::NodeSpec;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Windows {
    /// 竖排：一行一个候选。
    pub(crate) vertical: NodeSpec,

    /// 横排：候选排成一行。
    pub(crate) horizontal: NodeSpec,

    /// 手机键盘上方的候选条：不是浮窗，壳横向滚动它、把拼音行钉在视口上。
    pub(crate) strip: NodeSpec,
}
