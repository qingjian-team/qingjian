use std::sync::mpsc::{self, Receiver, Sender};

use qingjian_core::{FilledGloss, GlossFiller, Language};

use super::worker::GlossWorker;
use crate::chat_client::ChatClient;
use crate::config::PredictConfig;
use crate::error::PredictError;

/// 走网络的释义兜底。构造时起一个后台线程，`request` / `poll` 都只碰通道，不阻塞。
pub struct CloudGlossFiller {
    /// 往后台线程送词。
    requests: Sender<(Language, String)>,

    /// 从后台线程收释义。
    responses: Receiver<FilledGloss>,
}

impl CloudGlossFiller {
    /// 与云联想共用一份配置（接口、模型、密钥、超时）；没有密钥直接报错，壳退回 [`qingjian_core::NoGlossFiller`]。
    pub fn new(config: &PredictConfig) -> Result<Self, PredictError> {
        let api_key = config
            .resolve_api_key()
            .ok_or_else(|| PredictError::MissingApiKey(config.api_key_env.clone()))?;
        let client = ChatClient::new(config, api_key);
        let (requests, request_rx) = mpsc::channel();
        let (response_tx, responses) = mpsc::channel();
        let worker = GlossWorker::new(request_rx, response_tx, client);
        std::thread::Builder::new()
            .name("qingjian-gloss".to_owned())
            .spawn(move || {
                if let Err(error) = worker.run() {
                    tracing::error!(%error, "释义兜底线程退出");
                }
            })?;
        tracing::info!("释义兜底已启用");
        Ok(Self {
            requests,
            responses,
        })
    }
}

impl GlossFiller for CloudGlossFiller {
    fn request(&mut self, language: Language, word: &str) {
        if self.requests.send((language, word.to_owned())).is_err() {
            tracing::warn!("释义兜底线程已退出，请求被丢弃");
        }
    }

    fn poll(&mut self) -> Vec<FilledGloss> {
        // Empty 与 Disconnected 都当没有：线程退出时已经记过日志
        self.responses.try_iter().collect()
    }
}
