//! 实时翻译的独立配置，不复用云联想凭据。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LiveTranslationConfig {
    pub enabled: bool,

    pub base_url: String,

    pub model: String,

    pub debounce_ms: u64,

    pub timeout_ms: u64,
}

impl Default for LiveTranslationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: "http://localhost:11434".into(),
            model: "maternion/hy-mt2:1.8b".into(),
            debounce_ms: 180,
            timeout_ms: 5000,
        }
    }
}
