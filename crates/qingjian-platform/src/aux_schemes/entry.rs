//! 找到的一套辅码方案：只读了 `scheme.toml`，码表到装配时才读。

use std::path::PathBuf;

use qingjian_dictionary::AuxWordRules;

use crate::SchemeMeta;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuxSchemeEntry {
    /// 目录名；`[aux_code] disabled` 按它关。
    pub id: String,

    pub label: String,

    pub meta: SchemeMeta,

    pub table: PathBuf,

    pub words: AuxWordRules,

    /// 随包的方案（设置里不能移除）。
    pub bundled: bool,
}
