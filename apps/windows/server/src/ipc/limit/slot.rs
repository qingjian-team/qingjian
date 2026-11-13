//! 一条连接占着的名额，丢掉时还给 [`ConnectionLimit`]。

use std::sync::Arc;

use super::ConnectionLimit;

/// 跟着连接线程走，线程结束时丢掉。
pub struct Slot {
    limit: Arc<ConnectionLimit>,

    pid: u32,
}

impl Slot {
    pub(super) fn new(limit: Arc<ConnectionLimit>, pid: u32) -> Self {
        Self { limit, pid }
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        self.limit.release(self.pid);
    }
}
