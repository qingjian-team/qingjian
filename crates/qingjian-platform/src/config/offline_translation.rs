//! 本地选区翻译；只有用户在设置里下载模型并开启后才生效。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct OfflineTranslationConfig {
    pub enabled: bool,
}
