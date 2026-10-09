//! 四边的值：一个数四边相同，或 `[上, 右, 下, 左]`，与 CSS 同序。

use serde::Deserialize;

use super::length::Length;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(from = "EdgesRepr")]
pub(crate) struct Edges {
    pub(crate) top: Length,

    pub(crate) right: Length,

    pub(crate) bottom: Length,

    pub(crate) left: Length,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum EdgesRepr {
    All(Length),

    Each([Length; 4]),
}

impl From<EdgesRepr> for Edges {
    fn from(value: EdgesRepr) -> Self {
        let [top, right, bottom, left] = match value {
            EdgesRepr::All(length) => [length; 4],
            EdgesRepr::Each(edges) => edges,
        };
        Self {
            top,
            right,
            bottom,
            left,
        }
    }
}
