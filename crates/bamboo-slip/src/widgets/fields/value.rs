//! 一段的值：一段字，或使用方建好的节点（能点的链接）。

use taffy::NodeId;

pub enum FieldValue<'a> {
    Text(&'a str),

    Node(NodeId),
}
