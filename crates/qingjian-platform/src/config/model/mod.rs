mod local_model;
mod zhiyi_tier;

pub use local_model::LocalModel;
pub use zhiyi_tier::ZhiyiTier;

use serde::{Deserialize, Serialize};

/// 配置文件 `[model]` 分节：本地整句模型的开关。
///
/// 本地模型在本机给候选重新排序，全程离线、不联网，与云联想互不影响（本地先出、云端到了另占它自己的格）。
/// 选的模型还没下载时退回随包的含章·通变。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LocalModelConfig {
    /// 开着就加载模型、给整句重排。
    pub enabled: bool,

    /// 用哪个模型。
    pub name: LocalModel,

    /// 知意的档位，别的模型不看。
    pub zhiyi_tier: ZhiyiTier,

    /// 应用读不到光标前文时，这个应用里多少秒内上屏的字还给模型当前文；0 表示一直算。
    pub context_memory_seconds: u64,
}

impl Default for LocalModelConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            name: LocalModel::default(),
            zhiyi_tier: ZhiyiTier::default(),
            context_memory_seconds: qingjian_core::DEFAULT_FRESH_SECONDS,
        }
    }
}
