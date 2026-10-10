//! 后台线程：合并请求、防抖、限频、查缓存、发网络请求、回结果。

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use qingjian_core::{Prediction, PredictionRequest};

use crate::prompt::Reply;

use crate::cache::PredictionCache;
use crate::chat_client::ChatClient;
use crate::error::PredictError;

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
        }
    }

    /// 阻塞运行直到发送端全部关闭。
    pub fn run(mut self) -> Result<(), PredictError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let mut last_sent = None;
        while let Ok(first) = self.requests.recv() {
            let Some(request) = self.wait_for_request(first, last_sent) else {
                continue;
            };
            let key = PredictionCache::key(&request);
            if let Some(reply) = self.cache.get(&key) {
                tracing::debug!(sequence = request.sequence, kind = ?request.kind, "联想命中缓存");
                self.reply(request.sequence, reply.clone());
                continue;
            }
            let start = Instant::now();
            last_sent = Some(start);
            match runtime.block_on(self.client.complete(&request)) {
                Ok(reply) => {
                    tracing::info!(
                        sequence = request.sequence,
                        elapsed_ms = start.elapsed().as_millis(),
                        words = reply.words.len(),
                        sentence = reply.sentence.is_some(),
                        "联想完成"
                    );
                    // 空回复不进缓存：模型偶尔什么都不给（问字尤其），缓存住就等于这个问题以后永远没答案，
                    // 用户重打一遍也只会命中缓存（2026-09-07 `?mumumu` 就是这么卡住的）
                    if !reply.is_empty() {
                        self.cache.insert(key, reply.clone());
                    }
                    self.reply(request.sequence, reply);
                }
                Err(error) => {
                    tracing::warn!(sequence = request.sequence, %error, "联想失败");
                }
            }
        }
        Ok(())
    }

    /// 等到停键和间隔都满足，只保留最新请求；取消信号立即丢弃待发请求。
    /// 缓存命中只等停键，手动请求只等网络间隔。
    fn wait_for_request(
        &self,
        first: Option<PredictionRequest>,
        last_sent: Option<Instant>,
    ) -> Option<PredictionRequest> {
        let mut latest = first?;
        let mut changed = Instant::now();
        loop {
            let debounce = if latest.manual {
                Duration::ZERO
            } else {
                self.debounce.saturating_sub(changed.elapsed())
            };
            let interval = if self.cache.get(&PredictionCache::key(&latest)).is_some() {
                Duration::ZERO
            } else {
                last_sent.map_or(Duration::ZERO, |sent| {
                    self.min_interval.saturating_sub(sent.elapsed())
                })
            };
            match self.requests.recv_timeout(debounce.max(interval)) {
                Ok(Some(newer)) => {
                    latest = newer;
                    changed = Instant::now();
                }
                Ok(None) => return None,
                Err(RecvTimeoutError::Timeout) => return Some(latest),
                Err(RecvTimeoutError::Disconnected) => return None,
            }
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
