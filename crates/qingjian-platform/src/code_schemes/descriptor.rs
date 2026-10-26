//! `scheme.toml`：一套形码方案的名字、码表文件与按键规则。

use serde::Deserialize;

use crate::SchemeMeta;

/// 码表文件的缺省名，与 `scheme.toml` 同目录。
const DEFAULT_TABLE: &str = "table.tsv";

/// `scheme.toml` 的内容。没写的项取缺省：码表 `table.tsv`，自动上屏与顶字都开（与内置五笔一致）。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub(super) struct Descriptor {
    /// 方案种类：空着或 [`super::KIND`] 是形码；`shuangpin` 与 `aux` 是双拼与辅码方案（见 [`crate::shuangpin_schemes`]、[`crate::aux_schemes`]），形码这边跳过。
    #[serde(rename = "type")]
    pub kind: String,

    /// 界面与状态条上的名字；空着时用目录名。
    pub name: String,

    #[serde(flatten)]
    pub meta: SchemeMeta,

    /// 码表文件，相对方案目录。
    pub table: String,

    pub auto_commit: bool,

    pub top: bool,

    /// 满码几个字母；没写按码表里最常见的编码长度算。
    pub code_length: Option<usize>,
}

impl Default for Descriptor {
    fn default() -> Self {
        Self {
            kind: String::new(),
            name: String::new(),
            meta: SchemeMeta::default(),
            table: DEFAULT_TABLE.to_owned(),
            auto_commit: true,
            top: true,
            code_length: None,
        }
    }
}
