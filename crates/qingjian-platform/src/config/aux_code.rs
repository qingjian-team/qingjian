//! 配置文件 `[aux_code]` 分节：辅码开关与用哪一套码表。

use serde::{Deserialize, Serialize};

/// 配置文件 `[aux_code]` 分节：辅码开关与用哪一套码表。
///
/// 码表来自两处：用户目录 `codes/` 下导入的 `.qj`（按文件名，不含扩展名）与 `schemes/` 里的辅码方案（按 id）。
/// 同一时间只用一套：几套码混着筛，同一个字母在不同方案里意思不同，筛得反而松。
/// `enabled` 是整条辅码线的总开关（缺省关）：关着时 `;` 完全保持原生行为，开着但没有可用码表也不进辅码态。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AuxCodeConfig {
    /// 辅码总开关，缺省关。关着时触发键 `;` 完全保持原生行为（全角标点 / 双拼韵母键）。
    pub enabled: bool,

    /// 用哪一套（码表文件名或方案 id）；空着或找不到时用第一套没被关掉的。
    pub scheme: String,

    /// 老配置逐张关掉的码表。现在只在 `scheme` 没选时用来跳过，设置程序不再写它。
    pub disabled: Vec<String>,
}

impl AuxCodeConfig {
    /// 用户目录里的码表是否启用。
    pub fn is_enabled(&self, stem: &str) -> bool {
        !self.disabled.iter().any(|d| d == stem)
    }

    /// 从可用的几套（按列出的先后）里挑当前用的：选了且在就是它，否则第一套没被关掉的。
    pub fn pick<'a>(&self, ids: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
        let ids: Vec<&str> = ids.into_iter().collect();
        let chosen = self.scheme.trim();
        ids.iter()
            .find(|id| !chosen.is_empty() && **id == chosen)
            .or_else(|| ids.iter().find(|id| self.is_enabled(id)))
            .copied()
    }
}
