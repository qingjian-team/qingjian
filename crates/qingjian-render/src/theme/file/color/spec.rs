//! 节点上的颜色：一个颜色引用，或按条件二选一 `{ "if": "highlighted", "then": "@a", "else": "@b" }`
//! （条件写法同 `when`）。高亮的候选换白字这类「变体」就靠它，不用把节点抄两份。

use serde::Deserialize;

use super::ColorRef;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub(crate) enum ColorSpec {
    /// 固定的颜色。
    Fixed(ColorRef),

    /// 条件成立用 `then`，否则用 `otherwise`。
    Switch {
        #[serde(rename = "if")]
        condition: String,

        then: ColorRef,

        #[serde(rename = "else")]
        otherwise: ColorRef,
    },
}

impl ColorSpec {
    /// 两个分支的颜色引用（校验用）。
    pub(crate) fn refs(&self) -> Vec<&ColorRef> {
        match self {
            Self::Fixed(color) => vec![color],
            Self::Switch {
                then, otherwise, ..
            } => vec![then, otherwise],
        }
    }
}
