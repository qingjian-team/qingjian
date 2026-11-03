//! 高亮片段的种类；壳按种类上色。

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CodeKind {
    Keyword,
    String,
    Comment,
    Number,
}
