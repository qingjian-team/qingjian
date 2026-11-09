//! 节点的点击动作（JSON 里的 `action`）：写了的节点连同子树成为点击区域，点中交给壳。

use serde::Deserialize;

use crate::renderer::HitTarget;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum NodeAction {
    PreviousPage,
    NextPage,
}

impl NodeAction {
    pub(crate) fn target(self) -> HitTarget {
        match self {
            Self::PreviousPage => HitTarget::Page(-1),
            Self::NextPage => HitTarget::Page(1),
        }
    }
}
