//! 后台线程：收请求、防抖、查缓存、发网络请求、回结果。

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use qingjian_core::{Prediction, PredictionKind, PredictionRequest};

use crate::prompt::Reply;

use crate::cache::PredictionCache;
use crate::chat_client::ChatClient;
use crate::error::PredictError;

/// 缓存条数。
const CACHE_CAPACITY: usize = 64;

pub struct Worker {
    /// 请求入口。主线程 drop 掉发送端后线程自然退出。
    requests: Receiver<PredictionRequest>,

    /// 结果出口。
    responses: Sender<Prediction>,

    /// 网络客户端。
    client: ChatClient,

    /// 防抖窗口。
    debounce: Duration,

    /// 结果缓存。
    cache: PredictionCache,
}

/// 缓存里存的是解析后的回复。
type Cached = Reply;

impl Worker {
    pub fn new(
        requests: Receiver<PredictionRequest>,
        responses: Sender<Prediction>,
        client: ChatClient,
        debounce: Duration,
    ) -> Self {
        Self {
            requests,
            responses,
            client,
            debounce,
            cache: PredictionCache::with_capacity(CACHE_CAPACITY),
        }
    }

    /// 阻塞运行直到发送端全部关闭。
    pub fn run(mut self) -> Result<(), PredictError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        while let Ok(first) = self.requests.recv() {
            let Some(requests) = self.debounce(first) else {
                return Ok(());
            };
            let mut pending = Vec::new();
            for request in requests {
                let key = PredictionCache::key(&request);
                if let Some(reply) = self.cache.get(&key) {
                    tracing::debug!(sequence = request.sequence, kind = ?request.kind, "预测命中缓存");
                    self.reply(request.sequence, reply.clone());
                } else {
                    pending.push(request);
                }
            }
            let start = Instant::now();
            match pending.as_slice() {
                [] => {}
                [request] => {
                    let reply = runtime.block_on(self.client.complete(request));
                    self.finish(request, reply, start);
                }
                [first, second] => {
                    let (first_reply, second_reply) = runtime.block_on(async {
                        tokio::join!(self.client.complete(first), self.client.complete(second),)
                    });
                    self.finish(first, first_reply, start);
                    self.finish(second, second_reply, start);
                }
                _ => unreachable!("防抖只保留一条组句请求和一条翻译请求"),
            }
        }
        Ok(())
    }

    /// 防抖：在窗口内持续收到新请求就一直等；组句和翻译各保留最后一个，并发发出。
    fn debounce(&self, first: PredictionRequest) -> Option<Vec<PredictionRequest>> {
        let mut composition = None;
        let mut translation = None;
        match first.kind {
            PredictionKind::Translate => translation = Some(first),
            PredictionKind::Compose | PredictionKind::Question => composition = Some(first),
        }
        loop {
            match self.requests.recv_timeout(self.debounce) {
                Ok(newer) => match newer.kind {
                    PredictionKind::Translate => translation = Some(newer),
                    PredictionKind::Compose | PredictionKind::Question => composition = Some(newer),
                },
                Err(RecvTimeoutError::Timeout) => {
                    return Some(translation.into_iter().chain(composition).collect());
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
                    "预测完成"
                );
                // 空回复不进缓存：模型偶尔什么都不给，缓存住会令重试永远没有答案。
                if !reply.is_empty() {
                    self.cache
                        .insert(PredictionCache::key(request), reply.clone());
                }
                self.reply(request.sequence, reply);
            }
            Err(error) => tracing::warn!(sequence = request.sequence, %error, "预测失败"),
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
