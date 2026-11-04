//! 单个推理线程；待处理选区只留最新一条，取消与丢弃不等待模型结束。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, mpsc::Sender};
use std::time::{Duration, Instant};

use qingjian_core::{Prediction, PredictionRequest};

use crate::{Backend, OfflineError};

pub(super) struct Shared {
    pub pending: Mutex<Option<(u64, PredictionRequest)>>,

    pub changed: Condvar,

    pub generation: AtomicU64,

    pub stopped: AtomicBool,
}

impl Default for Shared {
    fn default() -> Self {
        Self {
            pending: Mutex::new(None),
            changed: Condvar::new(),
            generation: AtomicU64::new(0),
            stopped: AtomicBool::new(false),
        }
    }
}

pub(super) fn run(
    shared: Arc<Shared>,
    replies: Sender<Prediction>,
    models: PathBuf,
    runtime: PathBuf,
) {
    let mut loaded = HashMap::<String, Backend>::new();
    loop {
        let Ok(mut slot) = shared.pending.lock() else {
            return;
        };
        while slot.is_none() && !shared.stopped.load(Ordering::Acquire) {
            let Ok(next) = shared.changed.wait(slot) else {
                return;
            };
            slot = next;
        }
        if shared.stopped.load(Ordering::Acquire) {
            return;
        }
        let Some((generation, request)) = slot.take() else {
            continue;
        };
        drop(slot);
        let deadline = Instant::now() + Duration::from_secs(60);
        let obsolete = || {
            shared.generation.load(Ordering::Acquire) != generation
                || shared.stopped.load(Ordering::Acquire)
        };
        let cancelled = || obsolete() || Instant::now() >= deadline;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let direction = match request.target_language.as_str() {
                "en" => "zh-en",
                "zh" => "en-zh",
                _ => return Err(OfflineError::Invalid("unsupported target")),
            };
            let backend = match loaded.entry(direction.to_owned()) {
                std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
                std::collections::hash_map::Entry::Vacant(entry) => {
                    let backend = Backend::load(&models.join(direction), &runtime)?;
                    tracing::info!(direction, "本地翻译模型已加载");
                    entry.insert(backend)
                }
            };
            backend.translate(&request.text, &request.target_language, cancelled)
        }))
        .unwrap_or_else(|_| {
            loaded.clear();
            Err(OfflineError::Invalid("model inference panicked"))
        });
        // 用户取消的结果直接丢弃；超时仍需回报失败，让评审结束等待。
        if obsolete() {
            continue;
        }
        let sentence = match result {
            Ok(text) => Some(text),
            Err(error) => {
                tracing::warn!(%error, "本地选区翻译失败");
                None
            }
        };
        if replies
            .send(Prediction {
                sequence: request.sequence,
                words: Vec::new(),
                sentence,
            })
            .is_err()
        {
            return;
        }
    }
}
