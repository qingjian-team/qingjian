//! 接口配置。

/// 豆包流式语音识别的配置：新版控制台的 API Key 与资源 ID（计费方式）。不派生 `Debug`，免得密钥被打进日志。
#[derive(Clone)]
pub struct VolcengineConfig {
    pub api_key: String,

    /// `X-Api-Resource-Id`：[`Self::HOURLY`] 或 [`Self::CONCURRENT`]。
    pub resource_id: String,

    /// WebSocket 地址；缺省是只在结果有变化时才回包的优化版双向流式接口。
    pub endpoint: String,

    /// 自动加标点。
    pub punctuation: bool,
}

impl VolcengineConfig {
    /// 豆包流式语音识别模型 2.0，小时版（按时长计费）。
    pub const HOURLY: &str = "volc.seedasr.sauc.duration";

    /// 同一模型的并发版（按并发路数计费）。
    pub const CONCURRENT: &str = "volc.seedasr.sauc.concurrent";

    pub const ENDPOINT: &str = "wss://openspeech.bytedance.com/api/v3/sauc/bigmodel_async";

    pub fn new(api_key: impl Into<String>, resource_id: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into().trim().to_owned(),
            resource_id: resource_id.into().trim().to_owned(),
            endpoint: Self::ENDPOINT.to_owned(),
            punctuation: true,
        }
    }

    pub fn ready(&self) -> bool {
        !self.api_key.is_empty() && !self.resource_id.is_empty()
    }
}
