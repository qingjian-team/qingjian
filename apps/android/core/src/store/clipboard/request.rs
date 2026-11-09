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

    /// 把原文是 `from` 的那条改成 `text`，改完挪到最前（同桌面剪贴板的「编辑」）；图片、空白的不改。
    /// `index` 是打开编辑时它在第几条：编辑期间同步或复制进来新的会把它往后挤，所以按原文核对、对不上再按原文找。
    Edit {
        index: usize,
        from: String,
        text: String,
    },

    Clear,
}
