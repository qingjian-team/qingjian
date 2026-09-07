//! 释义生成：从一元词频表挑词，交给通用批量驱动。

use std::sync::Arc;
use std::time::Duration;

use crate::args::GenerateArgs;
use crate::batch::{self, Task};
use crate::client::LlmClient;
use crate::entry::GlossEntry;
use crate::error::GlossError;
use crate::prompt;
use crate::store::Store;
use crate::words;

/// 释义任务。
struct GlossTask;

impl Task for GlossTask {
    type Entry = GlossEntry;

    fn system_prompt(&self) -> &str {
        prompt::SYSTEM_PROMPT
    }

    fn user_prompt(&self, words: &[String]) -> String {
        prompt::user_prompt(words)
    }

    fn parse_reply(
        &self,
        content: &str,
        words: &[String],
    ) -> Result<Vec<GlossEntry>, serde_json::Error> {
        prompt::parse_reply(content, words)
    }
}

pub async fn run(args: GenerateArgs) -> Result<(), GlossError> {
    let selected = words::select(&args.words, args.min_count, args.max_chars, args.limit)?;
    let store = Arc::new(Store::<GlossEntry>::open(&args.out)?);
    let pending: Vec<String> = selected
        .iter()
        .filter(|w| !store.contains(w))
        .cloned()
        .collect();
    tracing::info!(
        selected = selected.len(),
        already_done = store.len(),
        pending = pending.len(),
        model = %args.model,
        "开始生成释义"
    );
    if pending.is_empty() {
        return Ok(());
    }
    let client = Arc::new(LlmClient::new(
        &args.base_url,
        &args.api_key,
        &args.model,
        Duration::from_secs(args.timeout_secs),
    ));
    batch::run(
        Arc::new(GlossTask),
        client,
        store,
        pending,
        args.batch,
        args.concurrency,
    )
    .await;
    Ok(())
}
