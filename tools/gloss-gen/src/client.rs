use std::time::Duration;

use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::types::chat::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
    ChatCompletionRequestUserMessage, CreateChatCompletionRequestArgs, ReasoningEffort,
    ResponseFormat,
};

use crate::error::GlossError;

/// 一批词的回复上限：40 个词、每个不到 100 token。
const MAX_TOKENS: u32 = 6000;

/// 词典条目要稳定。
const TEMPERATURE: f32 = 0.2;

/// OpenAI 兼容聊天接口：一段 system + 一段 user 进，JSON 文本出。
pub struct LlmClient {
    /// 底层客户端。
    client: Client<OpenAIConfig>,

    /// 模型名。
    model: String,

    /// 单个请求超时。
    timeout: Duration,
}

impl LlmClient {
    pub fn new(base_url: &str, api_key: &str, model: &str, timeout: Duration) -> Self {
        let config = OpenAIConfig::new()
            .with_api_base(base_url.trim_end_matches('/'))
            .with_api_key(api_key);
        Self {
            client: Client::with_config(config),
            model: model.to_owned(),
            timeout,
        }
    }

    pub async fn complete(&self, system: &str, user: &str) -> Result<String, GlossError> {
        let messages: Vec<ChatCompletionRequestMessage> = vec![
            ChatCompletionRequestSystemMessage::from(system).into(),
            ChatCompletionRequestUserMessage::from(user).into(),
        ];
        // DeepSeek V4 缺省思考，词典条目不需要，关掉省钱省时间
        let body = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(messages)
            .max_tokens(MAX_TOKENS)
            .temperature(TEMPERATURE)
            .response_format(ResponseFormat::JsonObject)
            .reasoning_effort(ReasoningEffort::None)
            .build()?;
        let response = tokio::time::timeout(self.timeout, self.client.chat().create(body))
            .await
            .map_err(|_| GlossError::Timeout(self.timeout.as_secs()))??;
        response
            .choices
            .into_iter()
            .find_map(|choice| choice.message.content.filter(|c| !c.trim().is_empty()))
            .ok_or(GlossError::EmptyReply)
    }
}
