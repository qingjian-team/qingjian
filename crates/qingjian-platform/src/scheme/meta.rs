//! 方案的元信息：`scheme.toml` 里形码与双拼方案共有的那几项，分享方案时让人知道是谁的、什么版本、能不能再分发。

use serde::Deserialize;

/// 每一项都可省略。
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct SchemeMeta {
    pub author: String,

    pub version: String,

    /// 一两句话说这套方案是什么。
    pub description: String,

    /// 方案（尤其是码表）的许可，如 `Apache-2.0`。
    pub license: String,
}
