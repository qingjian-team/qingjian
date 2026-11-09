//! 辅码方案的 `scheme.toml`：名字、码表文件与词码规则。

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::SchemeMeta;

/// 码表文件的缺省名，与 `scheme.toml` 同目录。
const DEFAULT_TABLE: &str = "table.tsv";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub(super) struct Descriptor {
    /// 必须是 [`super::KIND`]。
    #[serde(rename = "type")]
    pub kind: String,

    /// 设置里显示的名字；空着时用目录名。
    pub name: String,

    #[serde(flatten)]
    pub meta: SchemeMeta,

    /// 码表文件，相对方案目录：`字\t码` 的 TSV、Rime `.dict.yaml` 或导入过的 `.qj`。
    pub table: String,

    /// 词长 → 公式（`"2" = "AaBa"`），码表里没有的词按它从单字的码拼；最长的那条也管更长的词。
    pub words: BTreeMap<String, String>,
}

impl Default for Descriptor {
    fn default() -> Self {
        Self {
            kind: String::new(),
            name: String::new(),
            meta: SchemeMeta::default(),
            table: DEFAULT_TABLE.to_owned(),
            words: BTreeMap::new(),
        }
    }
}
