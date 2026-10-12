use thiserror::Error;

#[derive(Debug, Error)]
pub enum PredictError {
    #[error("no API key: set `api_key` in config or the `{0}` environment variable")]
    MissingApiKey(String),

    #[error("failed to start async runtime: {0}")]
    Runtime(#[from] std::io::Error),

    #[error("API request failed: {0}")]
    Api(#[from] async_openai::error::OpenAIError),

    #[error("API request timed out after {0} ms")]
    Timeout(u64),

    #[error("API returned no usable content")]
    EmptyReply,

    /// 正文为空且回复因长度截断：模型开着思考，输出额度花在思考上了。
    #[error("API reply was cut off before any content (output budget spent on reasoning)")]
    BudgetExhausted,

    #[error("failed to encode request: {0}")]
    Encode(#[from] serde_json::Error),
}

impl PredictError {
    /// 账户层面的失败：密钥无效（401）、余额不足（402）、没有权限（403）。充值或换配置之前每次都会一样失败。
    pub fn is_account_failure(&self) -> bool {
        matches!(
            self,
            Self::Api(async_openai::error::OpenAIError::ApiError(response))
                if matches!(response.status_code.as_u16(), 401..=403)
        )
    }
}
