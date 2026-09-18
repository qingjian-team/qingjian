//! 配置文件 `[aux_code]` 分节：辅码码表的开关。

use serde::{Deserialize, Serialize};

/// 配置文件 `[aux_code]` 分节：辅码码表的开关。
///
/// 与 `[dictionaries]` 同形：用户目录 `codes/` 下的 `.qj` 文件在就加载，只有列在 `disabled` 里的
/// （按文件名，不含扩展名）跳过；导入 / 移除就是加 / 删文件。
/// `enabled` 是整条辅码线的总开关（缺省关）：关着时 `;` 完全保持原生行为，开着但没有任何
/// 可用码表也不进辅码态。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AuxCodeConfig {
    /// 辅码总开关，缺省关。关着时触发键 `;` 完全保持原生行为（全角标点 / 双拼韵母键）。
    pub enabled: bool,

    /// 关掉的码表（文件名，不含 `.qj`）。
    pub disabled: Vec<String>,
}

impl AuxCodeConfig {
    /// 用户目录里的码表是否启用。
    pub fn is_enabled(&self, stem: &str) -> bool {
        !self.disabled.iter().any(|d| d == stem)
    }
}
