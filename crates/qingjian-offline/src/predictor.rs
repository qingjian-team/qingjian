//! Core 的非阻塞翻译提供方；只接显式选区请求，不处理普通候选联想。

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, mpsc};

use qingjian_core::{Prediction, PredictionKind, PredictionPolicy, PredictionRequest, Predictor};

use crate::worker::{self, Shared};

pub struct NativePredictor {
    shared: Arc<Shared>,

    replies: mpsc::Receiver<Prediction>,
}

impl NativePredictor {
    pub fn new(models: PathBuf, runtime: PathBuf) -> std::io::Result<Self> {
        let shared = Arc::new(Shared::default());
        let (sender, replies) = mpsc::channel();
        let state = Arc::clone(&shared);
        std::thread::Builder::new()
            .name("qingjian-offline-translation".into())
            .spawn(move || worker::run(state, sender, models, runtime))?;
        Ok(Self { shared, replies })
    }
}

impl Predictor for NativePredictor {
    fn policy(&self) -> PredictionPolicy {
        PredictionPolicy {
            automatic: false,
            before: 0,
            after: 0,
            slots: 0,
            ..PredictionPolicy::default()
        }
    }

    fn submit(&mut self, request: PredictionRequest) {
        if request.kind != PredictionKind::Translate || !request.manual {
            return;
        }
        if let Ok(mut slot) = self.shared.pending.lock() {
            let generation = self.shared.generation.fetch_add(1, Ordering::AcqRel) + 1;
            *slot = Some((generation, request));
            self.shared.changed.notify_one();
        }
    }

    fn cancel(&mut self) {
        self.shared.generation.fetch_add(1, Ordering::AcqRel);
        if let Ok(mut slot) = self.shared.pending.lock() {
            *slot = None;
        }
    }

    fn poll(&mut self) -> Option<Prediction> {
        self.replies.try_recv().ok()
    }
}

impl Drop for NativePredictor {
    fn drop(&mut self) {
        // 与 worker 的等待共用锁，避免「刚检查退出标记、还没 wait」时丢唤醒。
        if let Ok(mut slot) = self.shared.pending.lock() {
            *slot = None;
            self.shared.stopped.store(true, Ordering::Release);
            self.shared.changed.notify_one();
        }
    }
}

#[cfg(test)]
mod tests;
