//! 后台线程：收请求、防抖、查缓存、发网络请求、回结果。

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use qingjian_core::{Prediction, PredictionRequest};

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

    phrases: crate::phrase_store::PhraseStore,
}

/// 缓存里存的是解析后的回复。
type Cached = Reply;

impl Worker {
    pub fn new(
        requests: Receiver<PredictionRequest>,
        responses: Sender<Prediction>,
        client: ChatClient,
        debounce: Duration,
        namespace: u64,
    ) -> Self {
        Self {
            requests,
            responses,
            client,
            debounce,
            cache: PredictionCache::with_capacity(CACHE_CAPACITY),
            phrases: crate::phrase_store::PhraseStore::load(namespace),
        }
    }

    /// 阻塞运行直到发送端全部关闭。
    pub fn run(mut self) -> Result<(), PredictError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let mut pending = None;
        while let Some(first) = pending.take().or_else(|| self.requests.recv().ok()) {
            if let Some(reply) = self.cached_translation(&first) {
                self.reply(first.sequence, reply);
                if !self.client.cloud_association_for(&first) {
                    continue;
                }
            }
            let Some(request) = self.debounce(first) else {
                return Ok(());
            };
            // 连续输入时末次请求在防抖期间到达，也必须先查离线词库。
            let cached = self.cached_translation(&request);
            if let Some(reply) = &cached {
                self.reply(request.sequence, reply.clone());
                if !self.client.cloud_association_for(&request) {
                    continue;
                }
            }
            let key = PredictionCache::key(&request);
            if let Some(reply) = self.cache.get(&key) {
                tracing::debug!(sequence = request.sequence, kind = ?request.kind, "联想命中缓存");
                self.reply(request.sequence, reply.clone());
                continue;
            }
            let start = Instant::now();
            let mut network_request = request.clone();
            if cached.is_some() {
                // 已有当前语言译文时，同一次云请求仅补中文联想，避免重复翻译费用。
                network_request.translation_source = None;
                network_request.target_language.clear();
            }
            // 新的输入取消旧的网络 future，避免过时请求挡住游戏输入。
            let result = runtime.block_on(async {
                let future = self.client.complete(&network_request);
                tokio::pin!(future);
                loop {
                    tokio::select! {
                        result = &mut future => break Some(result),
                        _ = tokio::time::sleep(Duration::from_millis(30)) => {
                            if let Ok(mut newer) = self.requests.try_recv() {
                                while let Ok(last) = self.requests.try_recv() { newer = last; }
                                // 保留最新请求，用下轮的 pending 槽而非重新放回通道。
                                pending = Some(newer);
                                break None;
                            }
                        }
                    }
                }
            });
            let Some(result) = result else {
                continue;
            };
            match result {
                Ok(mut reply) => {
                    // 缓存译文不能把本次空中文回复变成可缓存结果。
                    let cacheable = !reply.is_empty();
                    if let Some(cached) = &cached {
                        reply.translation.clone_from(&cached.translation);
                    }
                    self.phrases.remember(&reply.translation);
                    tracing::info!(
                        sequence = request.sequence,
                        elapsed_ms = start.elapsed().as_millis(),
                        words = reply.words.len(),
                        sentence = reply.sentence.is_some(),
                        "联想完成"
                    );
                    // 空回复不进缓存：模型偶尔什么都不给（问字尤其），缓存住就等于这个问题以后永远没答案，
                    // 用户重打一遍也只会命中缓存（2026-09-07 `?mumumu` 就是这么卡住的）
                    if cacheable {
                        self.cache.insert(key, reply.clone());
                    }
                    self.reply(request.sequence, reply);
                }
                Err(error) => {
                    if !matches!(error, PredictError::OfflineOnly) {
                        tracing::warn!(sequence = request.sequence, "联想或翻译请求失败，原文保留");
                    }
                    let _ = self.responses.send(Prediction {
                        sequence: request.sequence,
                        failed: true,
                        translation: cached.map(|reply| reply.translation).unwrap_or_default(),
                        ..Prediction::default()
                    });
                }
            }
        }
        Ok(())
    }

    /// 防抖：在窗口内持续收到新请求就一直等，只保留最后一个。发送端关闭返回 `None`。
    fn debounce(&self, mut latest: PredictionRequest) -> Option<PredictionRequest> {
        loop {
            match self.requests.recv_timeout(self.debounce) {
                Ok(newer) => {
                    latest = newer;
                    if self.phrases.lookup(&latest).is_some()
                        && !self.client.cloud_association_for(&latest)
                    {
                        return Some(latest);
                    }
                }
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
            translation: reply.translation,
            failed: false,
        });
    }

    fn cached_translation(&self, request: &PredictionRequest) -> Option<Reply> {
        let t = self.phrases.lookup(request)?;
        crate::accuracy::valid(
            &t.source,
            &t.text,
            t.language.code(),
            self.client.terminology(),
        )
        .then_some(Reply {
            translation: Some(t),
            ..Reply::default()
        })
    }
}

#[cfg(test)]
mod tests;
