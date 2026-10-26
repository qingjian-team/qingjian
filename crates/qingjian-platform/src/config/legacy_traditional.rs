//! 旧键 `[general] traditional` 的两种写法：0.2.1 及以前是布尔，
//! 繁体三档的开发版（#625 到 533e09b 之间）写过字符串。两种都读得进，配置才不会因为这一项整个解析失败。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LegacyTraditional {
    /// `traditional = true`：台湾字形。
    Flag(bool),

    /// `traditional = "hongkong"`：与 `traditional_variant` 同一套写法。
    Key(String),
}
