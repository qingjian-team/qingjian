//! 壳交过来的语音识别配置（JSON，含解好的密钥），按它挑识别方。

use qingjian_core::{NoTranscriber, Transcriber};
use qingjian_voice::{VolcengineConfig, VolcengineTranscriber};
use serde::Deserialize;

/// 设置页「语音识别」里填的（Kotlin 解出密钥后整份交过来）。不派生 `Debug`，免得密钥被打进日志。
#[derive(Deserialize)]
pub struct VoiceSettings {
    /// 识别服务：`volcengine`；别的值当没选。
    pub provider: String,

    pub api_key: String,

    /// 火山引擎的资源 ID（计费方式）。
    #[serde(default)]
    pub resource_id: String,
}

impl VoiceSettings {
    pub fn volcengine(&self) -> Option<VolcengineConfig> {
        if self.provider != PROVIDER_VOLCENGINE {
            return None;
        }
        let resource = if self.resource_id.trim().is_empty() {
            VolcengineConfig::HOURLY
        } else {
            self.resource_id.as_str()
        };
        Some(VolcengineConfig::new(self.api_key.as_str(), resource))
    }

    /// 这份配置对应的识别方；没选服务是 `NoTranscriber`（没填密钥的 `available()` 为假，入口照样提示还不能用）。
    pub fn transcriber(&self) -> Box<dyn Transcriber> {
        match self.volcengine() {
            Some(config) => Box::new(VolcengineTranscriber::new(config)),
            None => Box::new(NoTranscriber),
        }
    }
}

const PROVIDER_VOLCENGINE: &str = "volcengine";
