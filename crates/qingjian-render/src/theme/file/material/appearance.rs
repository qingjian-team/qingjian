//! `material.appearance`：文字深浅跟系统外观，还是跟着材质随背后内容反色。

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum MaterialAppearance {
    System,

    Auto,
}
