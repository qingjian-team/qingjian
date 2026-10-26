//! 一条常用语。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Phrase {
    pub text: String,

    /// 所在分组（「目录」）。
    pub group: String,

    /// 输入码：小写字母，空为没有。
    #[serde(default)]
    pub code: String,
}
