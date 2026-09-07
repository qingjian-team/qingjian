use std::path::PathBuf;

/// 一次导入的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Imported {
    /// 写出的 `.qj`。
    pub path: PathBuf,

    /// 词库名（Rime 头里的 `name:`，否则文件名主干）。
    pub name: String,

    /// 词条数。
    pub entries: usize,
}
