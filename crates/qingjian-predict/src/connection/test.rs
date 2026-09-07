//! 云服务连通性测试：偏好设置「云服务」页的「测试连接」按钮用。
//! 起一个线程用当前配置发一条最小的聊天请求，结果经通道回来；调用方在主线程轮询 [`ConnectionTest::poll`]。

use std::sync::mpsc;
use std::time::Instant;

use super::report::ConnectionReport;
use crate::chat_client::ChatClient;
use crate::config::PredictConfig;
use crate::error::PredictError;

/// 系统提示：告诉模型这只是探活。
const SYSTEM_PROMPT: &str = "你是输入法的连通性测试。";

/// 用户消息：要一个固定的 JSON，回复越短越好。
const USER_PROMPT: &str = "回复 JSON 对象 {\"ok\": true}，不要别的内容。";

/// 回复 token 上限：只要几个字。
const MAX_TOKENS: u32 = 32;

/// 一次进行中的连通性测试。
pub struct ConnectionTest {
    /// 测试线程送回的结果；线程只发一次。
    result: mpsc::Receiver<Result<ConnectionReport, PredictError>>,
}

impl ConnectionTest {
    /// 按配置发起测试。密钥缺失、线程起不来这两种情况当场报错，其余错误从 [`poll`](Self::poll) 回来。
    pub fn start(config: &PredictConfig) -> Result<Self, PredictError> {
        let api_key = config
            .resolve_api_key()
            .ok_or_else(|| PredictError::MissingApiKey(config.api_key_env.clone()))?;
        let client = ChatClient::new(config, api_key);
        let model = config.model.clone();
        let (sender, result) = mpsc::channel();
        std::thread::Builder::new()
            .name("qingjian-connection-test".to_owned())
            .spawn(move || {
                let outcome = run(&client, model);
                match &outcome {
                    Ok(report) => tracing::info!(
                        model = %report.model,
                        elapsed_ms = report.elapsed.as_millis() as u64,
                        reply = %report.reply,
                        "云服务连通性测试成功"
                    ),
                    Err(error) => tracing::warn!(%error, "云服务连通性测试失败"),
                }
                // 收的一方不在了（窗口关了、又点了一次）就算了
                let _ = sender.send(outcome);
            })?;
        Ok(Self { result })
    }

    /// 结果到了就取走；没到返回 `None`。取走之后再调永远是 `None`。
    pub fn poll(&self) -> Option<Result<ConnectionReport, PredictError>> {
        self.result.try_recv().ok()
    }
}

/// 在测试线程里跑：建一个单线程运行时，发一条请求，计时。
fn run(client: &ChatClient, model: String) -> Result<ConnectionReport, PredictError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let started = Instant::now();
    let reply = runtime.block_on(client.chat(SYSTEM_PROMPT, USER_PROMPT, MAX_TOKENS))?;
    Ok(ConnectionReport {
        model,
        elapsed: started.elapsed(),
        reply: reply.trim().to_owned(),
    })
}
