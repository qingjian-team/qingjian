//! [`Replica`]：同步看到的本机存储。Windows 的 SQLite 和安卓的 jsonl 各实现一份。

use crate::error::SyncError;
use crate::record::{Record, Stamp};

pub trait Replica {
    /// 本机所有条目的清单，含删除标记。
    fn stamps(&self) -> Result<Vec<Stamp>, SyncError>;

    /// 按 id 取一条；已经没有了为 `None`。
    fn record(&self, id: &str) -> Result<Option<Record>, SyncError>;

    /// 记下对方的一条（已验过内容和 id 对得上），保留它的修改时刻。
    fn apply(&mut self, record: Record) -> Result<(), SyncError>;
}
