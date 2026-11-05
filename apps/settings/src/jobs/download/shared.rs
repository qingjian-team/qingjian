//! 下载线程与窗口共用的槽：线程写进度、窗口读。

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use bamboo_slip::Waker;

use super::Stage;

pub(super) struct Shared {
    pub(super) stage: Mutex<Stage>,

    pub(super) cancel: AtomicBool,

    pub(super) waker: Option<Waker>,
}

impl Shared {
    pub(super) fn new(waker: Option<Waker>) -> Self {
        Self {
            stage: Mutex::new(Stage::Downloading { done: 0, total: 0 }),
            cancel: AtomicBool::new(false),
            waker,
        }
    }

    pub(super) fn set(&self, stage: Stage) {
        if let Ok(mut slot) = self.stage.lock() {
            *slot = stage;
        }
        if let Some(waker) = &self.waker {
            waker.wake();
        }
    }

    pub(super) fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}
