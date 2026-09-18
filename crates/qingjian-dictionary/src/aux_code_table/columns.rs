//! 码表文件里的列位置：Rime 的 `columns` 列表按名字定下标。

/// Rime `.dict.yaml` 缺省的三列，与 Rime 一致。
const DEFAULT_COLUMNS: [&str; 3] = ["text", "code", "weight"];

/// 码表文件里的列位置：`columns` 列表按名字定下标。表里没有 `code` 列就是纯词表。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Columns {
    /// 词在哪一列。
    pub text: usize,

    /// 码在哪一列；`None` 表示这是纯词表（`columns: [text, weight]`）。
    pub code: Option<usize>,
}

impl Default for Columns {
    fn default() -> Self {
        names_to_columns(&default_column_names())
    }
}

/// 缺省列名（`text` / `code` / `weight`）。
pub(super) fn default_column_names() -> Vec<String> {
    DEFAULT_COLUMNS.map(str::to_owned).to_vec()
}

/// 按 `columns` 列表里的名字定下标；列表里没有的名字就是 `None`。`text` 缺省在第 0 列。
pub(super) fn names_to_columns(names: &[String]) -> Columns {
    let index_of = |want: &str| names.iter().position(|name| name == want);
    Columns {
        text: index_of("text").unwrap_or(0),
        code: index_of("code"),
    }
}
