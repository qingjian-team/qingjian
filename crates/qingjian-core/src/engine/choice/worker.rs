//! 选词模型的后台线程。

use std::cell::Cell;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;

use super::{ChoiceModel, ChoiceRequest};

/// 后台算好的一次选词：请求原样带回，壳拿它对上缓存。
pub(crate) struct Chosen {
    pub serial: u64,
    pub request: ChoiceRequest,
    pub logits: Vec<f32>,
    pub ms: u64,
}

/// 选词模型的后台线程：一页候选要算几十毫秒，不能放在按键回调里。排队时只算最新的一条。
pub(crate) struct ChoiceWorker {
    jobs: Sender<(u64, ChoiceRequest)>,
    results: Receiver<Chosen>,
    handle: Option<JoinHandle<()>>,

    /// 最近一次请求的序号；0 是还没请求过。
    latest: Cell<u64>,
}

impl ChoiceWorker {
    pub fn spawn(model: Arc<dyn ChoiceModel>) -> Self {
        let (jobs, job_rx) = channel::<(u64, ChoiceRequest)>();
        let (result_tx, results) = channel::<Chosen>();
        let handle = std::thread::Builder::new()
            .name("qingjian-choice".to_owned())
            .spawn(move || {
                while let Ok(mut job) = job_rx.recv() {
                    while let Ok(newer) = job_rx.try_recv() {
                        job = newer;
                    }
                    let (serial, request) = job;
                    let texts: Vec<&str> = request.texts.iter().map(String::as_str).collect();
                    let started = std::time::Instant::now();
                    let logits = model.score(&request.context, &request.typed, &texts);
                    let ms = started.elapsed().as_millis() as u64;
                    tracing::debug!(texts = texts.len(), ms, "选词模型打分完成");
                    if result_tx
                        .send(Chosen {
                            serial,
                            request,
                            logits,
                            ms,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .ok();
        if handle.is_none() {
            tracing::warn!("起不了选词模型线程，本次不用模型");
        }
        Self {
            jobs,
            results,
            handle,
            latest: Cell::new(0),
        }
    }

    pub fn latest(&self) -> u64 {
        self.latest.get()
    }

    pub fn submit(&self, request: ChoiceRequest) {
        let serial = self.latest.get() + 1;
        self.latest.set(serial);
        if self.jobs.send((serial, request)).is_err() {
            tracing::warn!("选词模型线程已退出");
        }
    }

    pub fn poll(&self) -> Option<Chosen> {
        self.results.try_recv().ok()
    }
}

impl Drop for ChoiceWorker {
    fn drop(&mut self) {
        let _ = self.handle.take();
    }
}
