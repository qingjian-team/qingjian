//! 分组里的一项：一行设置，可带几行从属于它的设置（开了某项才有意义的细项）。

use taffy::NodeId;

use crate::ui::Key;

pub struct Branch {
    pub head: NodeId,

    /// 从属的行；[`Branch::expander`] 收着时使用方不用建，给空的就行。
    pub children: Vec<NodeId>,

    /// 有从属项、照 Windows 画成展开卡片时：卡片的 key（也是点箭头时交给使用方的）与是否展开。
    /// 别的外观从属项一直露着（macOS 缩进一级），不给。
    pub expander: Option<(Key, bool)>,
}

impl Branch {
    /// 没有从属项的一行。
    pub fn leaf(head: NodeId) -> Self {
        Self {
            head,
            children: Vec::new(),
            expander: None,
        }
    }
}
