//! 一帧建树时登记的可交互元素，[`super::Ui::finish`] 交出、[`super::Interaction::attach`] 收下。

use taffy::NodeId;

use super::{Interact, Key};

pub struct Elements<M>(pub(super) Vec<(Key, NodeId, Interact<M>)>);
