//! 后台线程：合并请求、防抖、限频、查缓存、发网络请求、回结果。

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use qingjian_core::{Prediction, PredictionKind, PredictionRequest};
use tokio::task::JoinSet;

use crate::prompt::Reply;

use crate::cache::PredictionCache;
use crate::chat_client::ChatClient;
use crate::error::PredictError;
use crate::suspension::Suspension;

#[cfg(test)]
mod tests;

/// 缓存条数。
const CACHE_CAPACITY: usize = 64;

pub struct Worker {
    /// 请求入口。主线程 drop 掉发送端后线程自然退出。
    requests: Receiver<Option<PredictionRequest>>,

    /// 结果出口。
    responses: Sender<Prediction>,

    /// 网络客户端。
    client: ChatClient,

    /// 防抖窗口。
    debounce: Duration,

    /// 两次网络调用之间的最短间隔，失败的调用也计入。
    min_interval: Duration,

    /// 结果缓存。
    cache: PredictionCache,

    /// 余额不足、密钥无效后的暂停。
    suspension: Suspension,
}

/// 缓存里存的是解析后的回复。
type Cached = Reply;

impl Worker {
    pub fn new(
        requests: Receiver<Option<PredictionRequest>>,
        responses: Sender<Prediction>,
        client: ChatClient,
        debounce: Duration,
        min_interval: Duration,
    ) -> Self {
        Self {
            requests,
            responses,
            client,
            debounce,
            min_interval,
            cache: PredictionCache::with_capacity(CACHE_CAPACITY),
            suspension: Suspension::default(),
        }
    }

    /// 阻塞运行直到发送端全部关闭。
    pub fn run(mut self) -> Result<(), PredictError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let mut last_sent = None;
        while let Ok(first) = self.requests.recv() {
            let Some(requests) = self.wait_for_requests(first, last_sent) else {
                continue;
            };
            let mut pending = Vec::new();
            for request in requests {
                let key = PredictionCache::key(&request);
                if let Some(reply) = self.cache.get(&key) {
                    tracing::debug!(sequence = request.sequence, kind = ?request.kind, "联想命中缓存");
                    self.reply(request.sequence, reply.clone());
                } else {
                    pending.push(request);
                }
            }
            let start = Instant::now();
            if !pending.is_empty() && self.suspension.active(start) {
                pending.iter().for_each(|request| self.give_up(request));
                continue;
            }
            last_sent = Some(start);
            match pending.as_slice() {
                [] => {}
                [request] => {
                    let reply = runtime.block_on(self.client.complete(request));
                    self.finish(request, reply, start);
                }
                requests => {
                    let mut jobs = runtime.block_on(async {
                        let mut jobs = JoinSet::new();
                        for request in requests.iter().cloned() {
                            let client = self.client.clone();
                            jobs.spawn(async move {
                                let result = client.complete(&request).await;
                                (request, result)
                            });
                        }
                        jobs
                    });
                    while let Some(result) = runtime.block_on(jobs.join_next()) {
                        let (request, reply) = result.expect("预测任务不应 panic");
                        self.finish(&request, reply, start);
                    }
                }
            }
        }
        Ok(())
    }

    /// 等到停键和间隔都满足。组句、普通翻译与用户触发的候选译文各保留最新一条，随后同时发出。
    fn wait_for_requests(
        &self,
        first: Option<PredictionRequest>,
        last_sent: Option<Instant>,
    ) -> Option<Vec<PredictionRequest>> {
        let first = first?;
        let mut composition = None;
        let mut translation = None;
        let mut candidate_translation = None;
        match first.kind {
            PredictionKind::Translate => translation = Some(first),
            PredictionKind::CandidateTranslation => candidate_translation = Some(first),
            PredictionKind::Compose | PredictionKind::Question => composition = Some(first),
        }
        let mut changed = Instant::now();
        loop {
            let requests = [&composition, &translation, &candidate_translation]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            let debounce = if requests.iter().any(|request| !request.manual) {
                self.debounce.saturating_sub(changed.elapsed())
            } else {
                Duration::ZERO
            };
            let interval = if requests
                .iter()
                .all(|request| self.cache.get(&PredictionCache::key(request)).is_some())
            {
                Duration::ZERO
            } else {
                last_sent.map_or(Duration::ZERO, |sent| {
                    self.min_interval.saturating_sub(sent.elapsed())
                })
            };
            match self.requests.recv_timeout(debounce.max(interval)) {
                Ok(Some(newer)) => {
                    match newer.kind {
                        PredictionKind::Translate => translation = Some(newer),
                        PredictionKind::CandidateTranslation => candidate_translation = Some(newer),
                        PredictionKind::Compose | PredictionKind::Question => {
                            composition = Some(newer)
                        }
                    }
                    changed = Instant::now();
                }
                Ok(None) => return None,
                Err(RecvTimeoutError::Timeout) => {
                    return Some(
                        candidate_translation
                            .into_iter()
                            .chain(translation)
                            .chain(composition)
                            .collect(),
                    );
                }
                Err(RecvTimeoutError::Disconnected) => return None,
            }
        }
    }

    fn finish(
        &mut self,
        request: &PredictionRequest,
        result: Result<Cached, PredictError>,
        started: Instant,
    ) {
        match result {
            Ok(reply) => {
                tracing::info!(
                    sequence = request.sequence,
                    elapsed_ms = started.elapsed().as_millis(),
                    words = reply.words.len(),
                    sentence = reply.sentence.is_some(),
                    "联想完成"
                );
                // 空回复不进缓存：模型偶尔什么都不给（问字尤其），缓存住就等于这个问题以后永远没答案，
                // 用户重打一遍也只会命中缓存（2026-09-07 `?mumumu` 就是这么卡住的）
                if !reply.is_empty() {
                    self.cache
                        .insert(PredictionCache::key(request), reply.clone());
                }
                self.reply(request.sequence, reply);
            }
            Err(error) => {
                if !self.suspension.record(&error, Instant::now(), "联想") {
                    tracing::warn!(sequence = request.sequence, %error, "联想失败");
                }
                self.give_up(request);
            }
        }
    }

    /// 没问成（出错、暂停中）：翻译是用户点了在等的，回一条空结果让壳当场说失败，不必干等到超时；联想照旧不回。
    fn give_up(&self, request: &PredictionRequest) {
        if matches!(
            request.kind,
            PredictionKind::Translate | PredictionKind::CandidateTranslation
        ) {
            self.reply(request.sequence, Cached::default());
        }
    }

    fn reply(&self, sequence: u64, reply: Cached) {
        // 接收端没了说明 Predictor 已经被 drop，线程随后也会退出
        let _ = self.responses.send(Prediction {
            sequence,
            words: reply.words,
            sentence: reply.sentence,
        });
    }
}
