use std::cell::Cell;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;

use crate::sentence::SentenceScorer;

use super::WakeSlot;
use super::{GENERATE_BEAM, GENERATE_MAX_CHARS};

/// 一次后台任务：两个条件与一批要打分的文本，外加可选的「直接按这段按键生成整句」。
/// 两件事合成一条任务是因为它们都在用户停顿后一起发出，而排队的任务只算最新一条。
struct Job {
    /// 第几次请求，原样带回结果里：壳靠它分辨收到的是不是最新这次请求的（见 [`Scored::serial`]）。
    serial: u64,

    context: String,
    keys: String,
    texts: Vec<String>,

    /// 要生成整句的那段按键（整段作用域）；`None` 就只打分。
    generate: Option<String>,
}

/// 后台算好的结果，与任务一一对应。
pub(crate) struct Scored {
    /// 对应哪次请求。打字比模型快时上一次的结果会先到，它不能算这一轮的结果。
    pub serial: u64,

    pub context: String,
    pub keys: String,
    pub texts: Vec<String>,
    pub scores: Vec<f64>,

    /// 生成用的按键与生成出来的整句，对应任务里的 `generate`。
    pub generated: Option<(String, Vec<String>)>,
}

/// 后台打分线程：模型前向要几十毫秒，不能放在按键回调里。
/// 任务排队时只算最新的一条（旧的对应已经过去的输入状态）；线程随本结构一起结束。
pub(crate) struct RescoreWorker {
    jobs: Sender<Job>,
    results: Receiver<Scored>,
    handle: Option<JoinHandle<()>>,

    /// 最近一次请求的序号；0 是还没请求过。
    latest: Cell<u64>,
}

impl RescoreWorker {
    pub fn spawn(scorer: Box<dyn SentenceScorer>, wake: WakeSlot) -> Self {
        let (jobs, job_rx) = channel::<Job>();
        let (result_tx, results) = channel::<Scored>();
        let handle = std::thread::Builder::new()
            .name("qingjian-rescore".to_owned())
            .spawn(move || {
                while let Ok(mut job) = job_rx.recv() {
                    // 攒了好几条只算最后一条
                    while let Ok(newer) = job_rx.try_recv() {
                        job = newer;
                    }
                    let texts: Vec<&str> = job.texts.iter().map(String::as_str).collect();
                    let started = std::time::Instant::now();
                    let scores = scorer.score(&job.context, &job.keys, &texts);
                    tracing::debug!(
                        texts = texts.len(),
                        context_chars = job.context.chars().count(),
                        keys = job.keys.len(),
                        ms = started.elapsed().as_millis(),
                        "神经重打分完成"
                    );
                    let generated = job.generate.map(|keys| {
                        let started = std::time::Instant::now();
                        let texts = scorer.generate(&keys, GENERATE_BEAM, GENERATE_MAX_CHARS);
                        tracing::debug!(
                            keys = keys.len(),
                            got = texts.len(),
                            ms = started.elapsed().as_millis(),
                            "整句生成完成"
                        );
                        (keys, texts)
                    });
                    let done = Scored {
                        serial: job.serial,
                        context: job.context,
                        keys: job.keys,
                        texts: job.texts,
                        scores,
                        generated,
                    };
                    if result_tx.send(done).is_err() {
                        break;
                    }
                    wake.ring();
                }
            })
            .ok();
        if handle.is_none() {
            tracing::warn!("起不了神经重打分线程，本次不用模型");
        }
        Self {
            jobs,
            results,
            handle,
            latest: Cell::new(0),
        }
    }

    /// 最近一次请求的序号。
    pub fn latest(&self) -> u64 {
        self.latest.get()
    }

    pub fn is_alive(&self) -> bool {
        self.handle.is_some()
    }

    pub fn submit(
        &self,
        context: String,
        keys: String,
        texts: Vec<String>,
        generate: Option<String>,
    ) {
        let serial = self.latest.get() + 1;
        self.latest.set(serial);
        if self
            .jobs
            .send(Job {
                serial,
                context,
                keys,
                texts,
                generate,
            })
            .is_err()
        {
            tracing::warn!("神经重打分线程已退出");
        }
    }

    /// 取一条打好的分；没有就 `None`。
    pub fn poll(&self) -> Option<Scored> {
        self.results.try_recv().ok()
    }
}

impl Drop for RescoreWorker {
    fn drop(&mut self) {
        // 关掉任务通道线程就会退出；不等它（模型可能正算到一半）
        let _ = self.handle.take();
    }
}
