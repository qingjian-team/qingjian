use serde::{Deserialize, Serialize};

/// 配置文件 `[model]` 分节：本地整句模型的开关。
///
/// 随包的字级小模型在本机给整句候选重新排序，全程离线、不联网，与云联想互不影响（本地先出、云端到了另占它自己的格）。
/// 模型文件不在包里（或用户目录 `model/` 里）时开关无效。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LocalModelConfig {
    /// 开着就加载模型、给整句重排。
    pub enabled: bool,
}

impl Default for LocalModelConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}
