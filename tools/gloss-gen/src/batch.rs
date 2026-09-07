//! 分批并发请求的通用驱动：一批词 → 提示词 → 模型 → 解析 → 落盘；没拿到的词再问，最多几轮。
//! 释义生成与拼音标注共用，差别只在提示词与解析函数。

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::client::LlmClient;
use crate::store::{Keyed, Store};

/// 一批词最多问几轮：第一轮整批，之后只问上一轮没答或答坏的词。
const MAX_ROUNDS: usize = 3;

/// 一种任务：系统提示词、用户消息怎么拼、回复怎么解析。
pub trait Task: Send + Sync + 'static {
    /// 一行结果的类型。
    type Entry: Keyed + Send + Sync + 'static;

    fn system_prompt(&self) -> &str;

    fn user_prompt(&self, words: &[String]) -> String;

    /// 只收请求过的词，坏的丢掉。
    fn parse_reply(
        &self,
        content: &str,
        words: &[String],
    ) -> Result<Vec<Self::Entry>, serde_json::Error>;
}

/// 并发跑完 `pending` 里的词。`batch` 每请求几个词，`concurrency` 同时几个请求。
pub async fn run<T: Task>(
    task: Arc<T>,
    client: Arc<LlmClient>,
    store: Arc<Store<T::Entry>>,
    pending: Vec<String>,
    batch: usize,
    concurrency: usize,
) {
    let semaphore = Arc::new(Semaphore::new(concurrency.max(1)));
    let done = Arc::new(AtomicUsize::new(0));
    let total = pending.len();
    let mut tasks = JoinSet::new();
    for chunk in pending.chunks(batch.max(1)) {
        let words: Vec<String> = chunk.to_vec();
        let (task, client, store, semaphore, done) = (
            task.clone(),
            client.clone(),
            store.clone(),
            semaphore.clone(),
            done.clone(),
        );
        tasks.spawn(async move {
            let _permit = semaphore.acquire().await;
            let got = process_batch(&*task, &client, &store, words).await;
            let finished = done.fetch_add(got, Ordering::Relaxed) + got;
            tracing::info!(done = finished, total, "进度");
        });
    }
    while let Some(result) = tasks.join_next().await {
        if let Err(error) = result {
            tracing::warn!(%error, "任务异常");
        }
    }
    tracing::info!(done = done.load(Ordering::Relaxed), total, "结束");
}

/// 处理一批词：请求、解析、落盘；没拿到的词再问，最多几轮。返回拿到的条目数。
async fn process_batch<T: Task>(
    task: &T,
    client: &LlmClient,
    store: &Store<T::Entry>,
    mut batch: Vec<String>,
) -> usize {
    let mut got = 0;
    for round in 1..=MAX_ROUNDS {
        if batch.is_empty() {
            break;
        }
        let user = task.user_prompt(&batch);
        let content = match client.complete(task.system_prompt(), &user).await {
            Ok(content) => content,
            Err(error) => {
                tracing::warn!(round, words = batch.len(), %error, "请求失败");
                tokio::time::sleep(Duration::from_secs(2 * round as u64)).await;
                continue;
            }
        };
        let entries = match task.parse_reply(&content, &batch) {
            Ok(entries) => entries,
            Err(error) => {
                tracing::warn!(round, %error, "回复不是合法 JSON");
                continue;
            }
        };
        if let Err(error) = store.append(&entries) {
            tracing::error!(%error, "写结果失败");
            return got;
        }
        got += entries.len();
        batch.retain(|w| !entries.iter().any(|e| e.key() == w));
        if !batch.is_empty() {
            tracing::debug!(round, missing = ?batch, "有词没答，再问");
        }
    }
    if !batch.is_empty() {
        tracing::warn!(missing = ?batch, "放弃");
    }
    got
}
