//! [`ReceivedItem`]：一轮同步收到的最新一条。

use crate::record::RecordKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivedItem {
    pub id: String,

    pub kind: RecordKind,
}
