//! 壳发来的剪贴板操作（JSON，`op` 区分）。

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum ClipboardRequest {
    List,

    /// 系统剪贴板里新复制的文字。
    Add {
        text: String,
    },

    /// 删第 `index` 条（新的在前数）。
    Remove {
        index: usize,
    },

    Clear,
}
