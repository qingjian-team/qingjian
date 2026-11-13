//! 按客户端进程限管道连接数（#699）：一个进程开再多连接也只占这么多服务线程，挤不掉别的应用。

mod slot;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub use self::slot::Slot;

/// 一个进程最多同时开几条连接。DLL 一个 UI 线程一条，开很多窗口的资源管理器也远到不了这个数。
pub const PER_PROCESS: usize = 64;

/// 各客户端进程当前开着的连接数。
#[derive(Default)]
pub struct ConnectionLimit {
    open: Mutex<HashMap<u32, usize>>,
}

impl ConnectionLimit {
    /// 进程 `pid` 再开一条连接；已到上限返回 `None`。拿到的 [`Slot`] 丢掉时自动还回去。
    pub fn admit(self: &Arc<Self>, pid: u32) -> Option<Slot> {
        let mut open = self
            .open
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let count = open.entry(pid).or_default();
        if *count >= PER_PROCESS {
            return None;
        }
        *count += 1;
        Some(Slot::new(Arc::clone(self), pid))
    }

    fn release(&self, pid: u32) {
        let mut open = self
            .open
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(count) = open.get_mut(&pid) {
            *count -= 1;
            if *count == 0 {
                open.remove(&pid);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{ConnectionLimit, PER_PROCESS};

    #[test]
    fn a_process_is_capped_and_gets_slots_back_on_drop() {
        let limit = Arc::new(ConnectionLimit::default());
        let slots: Vec<_> = (0..PER_PROCESS).map(|_| limit.admit(7).unwrap()).collect();
        assert!(limit.admit(7).is_none(), "第 65 条要拒");
        assert!(limit.admit(8).is_some(), "别的进程不受影响");
        drop(slots);
        assert!(limit.admit(7).is_some(), "连接断了名额要还回来");
    }
}
