//! 选词模型的后台线程。

use std::cell::Cell;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;

use super::finished::Finished;
use super::job::Job;
use super::{ChoiceModel, ChoiceRequest, ComposeRequest};
use crate::engine::rescoring::WakeSlot;

/// 选词模型的后台线程：一页候选要算几十毫秒，不能放在按键回调里。排队时打分与出字各只算最新的一条。
pub(crate) struct ChoiceWorker {
    jobs: Sender<(u64, Job)>,
    results: Receiver<(u64, Finished)>,
    handle: Option<JoinHandle<()>>,

    /// 最近一次请求的序号；0 是还没请求过。
    latest: Cell<u64>,
}

impl ChoiceWorker {
    pub fn spawn(model: Arc<dyn ChoiceModel>, wake: WakeSlot) -> Self {
        let (jobs, job_rx) = channel::<(u64, Job)>();
        let (result_tx, results) = channel::<(u64, Finished)>();
        let handle = std::thread::Builder::new()
            .name("qingjian-choice".to_owned())
            .spawn(move || {
                while let Ok(first) = job_rx.recv() {
                    let mut score = None;
                    let mut compose = None;
                    for (serial, job) in std::iter::once(first).chain(job_rx.try_iter()) {
                        match job {
                            Job::Score(request) => score = Some((serial, request)),
                            Job::Compose(requests) => compose = Some((serial, requests)),
                        }
                    }
                    let mut done = Vec::with_capacity(2);
                    if let Some((serial, request)) = score {
                        done.push((serial, run_score(&*model, request)));
                    }
                    if let Some((serial, requests)) = compose {
                        done.push((serial, run_compose(&*model, &requests)));
                    }
                    // 序号小的先交：壳只在最近一次请求的结果到了才重查
                    done.sort_by_key(|(serial, _)| *serial);
                    if done
                        .into_iter()
                        .any(|result| result_tx.send(result).is_err())
                    {
                        break;
                    }
                    wake.ring();
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

    pub fn submit(&self, job: Job) {
        let serial = self.latest.get() + 1;
        self.latest.set(serial);
        if self.jobs.send((serial, job)).is_err() {
            tracing::warn!("选词模型线程已退出");
        }
    }

    pub fn poll(&self) -> Option<(u64, Finished)> {
        self.results.try_recv().ok()
    }
}

fn run_score(model: &dyn ChoiceModel, request: ChoiceRequest) -> Finished {
    let texts: Vec<&str> = request.texts.iter().map(String::as_str).collect();
    let started = std::time::Instant::now();
    let logits = model.score(&request.context, &request.typed, &texts);
    let ms = started.elapsed().as_millis() as u64;
    tracing::debug!(texts = texts.len(), ms, "选词模型打分完成");
    Finished::Scored {
        request,
        logits,
        ms,
    }
}

fn run_compose(model: &dyn ChoiceModel, requests: &[ComposeRequest]) -> Finished {
    let started = std::time::Instant::now();
    let tables = requests
        .iter()
        .map(|request| {
            let syllables: Vec<(&str, bool)> = request
                .syllables
                .iter()
                .map(|(text, complete)| (text.as_str(), *complete))
                .collect();
            let table = model.compose(&request.context, &syllables).map(Arc::new);
            (request.key.clone(), table)
        })
        .collect();
    tracing::debug!(
        requests = requests.len(),
        ms = started.elapsed().as_millis() as u64,
        "选词模型出字完成"
    );
    Finished::Composed(tables)
}

impl Drop for ChoiceWorker {
    fn drop(&mut self) {
        let _ = self.handle.take();
    }
}
