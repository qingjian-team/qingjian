//! 下载进度与取消标记。

use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

use crate::jobs::model_download::Stage;
use bamboo_slip::Waker;

pub(super) struct Shared {
    pub stage: Mutex<Stage>,

    pub cancel: AtomicBool,

    waker: Option<Waker>,
}

impl Shared {
    pub fn new(waker: Option<Waker>) -> Self {
        Self {
            stage: Mutex::new(Stage::Downloading { done: 0, total: 0 }),
            cancel: AtomicBool::new(false),
            waker,
        }
    }

    pub fn set(&self, stage: Stage) {
        if let Ok(mut slot) = self.stage.lock() {
            *slot = stage;
        }
        if let Some(waker) = &self.waker {
            waker.wake();
        }
    }

    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Acquire)
    }
}
