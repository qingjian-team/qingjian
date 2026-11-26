//! `[code.<id>]`：用户在设置里改的某套形码方案的按键规则，盖过方案 `scheme.toml` 里写的。
//!
//! 存在配置而不是写回 `scheme.toml`：随包的方案在安装目录里是只读的，用户换码表或重新导入方案时这些改动也不丢。

use serde::{Deserialize, Serialize};

/// 没写的项为 `None`：纯形码按方案默认，混输规则关闭。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CodeRules {
    /// 满码唯一自动上屏。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_commit: Option<bool>,

    /// 顶字。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top: Option<bool>,

    /// 混输时满码唯一自动上屏，未配置时关闭。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mixed_auto_commit: Option<bool>,

    /// 混输时优先形码候选并在满码后顶字续输，未配置时关闭。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mixed_top: Option<bool>,
}
