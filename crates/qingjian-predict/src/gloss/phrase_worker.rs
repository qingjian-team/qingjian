//! 整段翻译的后台线程：攒一批上屏的中文，一次请求翻成学习语言，逐条回给主线程。

use std::collections::HashSet;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use qingjian_core::{Encounter, Language};

use super::phrase_prompt;
use crate::chat_client::ChatClient;
use crate::error::PredictError;
use crate::suspension::Suspension;

/// 收到第一段后最多等这么久再发：时间线不急，多攒几段省请求。
const BATCH_WAIT: Duration = Duration::from_secs(5);

/// 一批最多几段。
const BATCH_SIZE: usize = 10;

/// 回复的 token 上限：十段短句的译文够用。
const MAX_TOKENS: u32 = 1200;

/// 本进程内翻过的最多记这么多段，再多清空重记（只为不重复翻同一段，丢了无妨）。
const MAX_ASKED: usize = 4096;

pub struct PhraseWorker {
    requests: Receiver<(Language, String)>,

    responses: Sender<Encounter>,

    client: ChatClient,

    /// 本进程内翻过的 (语言, 中文)，不再翻。
    asked: HashSet<(Language, String)>,

    suspension: Suspension,
}

impl PhraseWorker {
    pub fn new(
        requests: Receiver<(Language, String)>,
        responses: Sender<Encounter>,
        client: ChatClient,
    ) -> Self {
        Self {
            requests,
            responses,
            client,
            asked: HashSet::new(),
            suspension: Suspension::default(),
        }
    }

    /// 阻塞运行直到发送端全部关闭。
    pub fn run(mut self) -> Result<(), PredictError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        while let Ok(first) = self.requests.recv() {
            let Some(batch) = self.collect(first) else {
                return Ok(());
            };
            let mut languages: Vec<Language> = Vec::new();
            for (language, _) in &batch {
                if !languages.contains(language) {
                    languages.push(*language);
                }
            }
            for language in languages {
                let texts: Vec<String> = batch
                    .iter()
                    .filter(|(l, _)| *l == language)
                    .map(|(_, text)| text.clone())
                    .collect();
                self.ask(&runtime, language, texts);
            }
        }
        Ok(())
    }

    fn collect(&mut self, first: (Language, String)) -> Option<Vec<(Language, String)>> {
        let deadline = Instant::now() + BATCH_WAIT;
        let mut batch = Vec::with_capacity(BATCH_SIZE);
        self.take(&mut batch, first);
        while batch.len() < BATCH_SIZE {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match self.requests.recv_timeout(remaining) {
                Ok(item) => self.take(&mut batch, item),
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => return None,
            }
        }
        Some(batch)
    }

    fn take(&mut self, batch: &mut Vec<(Language, String)>, item: (Language, String)) {
        if self.asked.len() >= MAX_ASKED {
            self.asked.clear();
        }
        if self.asked.insert(item.clone()) {
            batch.push(item);
        }
    }

    fn ask(&mut self, runtime: &tokio::runtime::Runtime, language: Language, texts: Vec<String>) {
        if texts.is_empty() {
            return;
        }
        let start = Instant::now();
        if self.suspension.active(start) {
            self.forget(language, &texts);
            return;
        }
        let system = phrase_prompt::system_prompt(language);
        let user = phrase_prompt::user_prompt(&texts);
        match runtime.block_on(self.client.chat(&system, &user, MAX_TOKENS)) {
            Ok(content) => {
                let translated = phrase_prompt::parse_reply(&content, language, &texts);
                tracing::info!(
                    language = language.code(),
                    asked = texts.len(),
                    translated = translated.len(),
                    elapsed_ms = start.elapsed().as_millis(),
                    "整段翻译完成"
                );
                for encounter in translated {
                    if self.responses.send(encounter).is_err() {
                        return;
                    }
                }
            }
            Err(error) => {
                if self.suspension.record(&error, Instant::now(), "整段翻译") {
                    self.forget(language, &texts);
                } else {
                    tracing::warn!(language = language.code(), asked = texts.len(), %error, "整段翻译失败");
                }
            }
        }
    }

    fn forget(&mut self, language: Language, texts: &[String]) {
        for text in texts {
            self.asked.remove(&(language, text.clone()));
        }
    }
}
