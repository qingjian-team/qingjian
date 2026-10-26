//! 壳发来的常用语操作（JSON，`op` 区分）；`index` 是在全部常用语里的位置。

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum PhraseRequest {
    List,

    Add {
        text: String,
        group: String,
        code: String,
    },

    Update {
        index: usize,
        text: String,
        group: String,
        code: String,
    },

    Remove {
        index: usize,
    },

    /// 移到最前（「移至首位」）。
    Top {
        index: usize,
    },
}
