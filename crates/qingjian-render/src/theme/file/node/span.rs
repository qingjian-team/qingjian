//! 表格里跨列：`row` 横跨整行（高亮条）。

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Span {
    Row,
}
