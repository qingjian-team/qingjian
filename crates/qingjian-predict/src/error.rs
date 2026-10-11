use thiserror::Error;

#[derive(Debug, Error)]
pub enum PredictError {
    #[error("offline phrase not found; select cloud manually for full translation")]
    OfflineOnly,
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

    #[error("API reply was truncated; original text was preserved")]
    TruncatedReply,

    #[error("translation source exceeds 20000 characters")]
    TranslationTooLong,

    #[error("failed to encode request: {0}")]
    Encode(#[from] serde_json::Error),
}
