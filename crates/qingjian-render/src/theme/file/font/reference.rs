//! 节点里的 `font`：`text.styles` 里的样式名，或在某个样式上改几项：
//! `{ "base": "candidate", "size": 18, "weight": "semibold", "style": "italic" }`。只改字号不写行高时，行高按字号等比缩放。
//! 字族不能在这里改：换字族要定义一个命名样式。

use serde::Deserialize;

use super::FontStyle;
use crate::theme::{FontSpec, FontWeight};

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum FontRef {
    Named(String),

    Inline {
        /// 在哪个样式上改；不写从缺省样式改。
        base: Option<String>,

        size: Option<f32>,

        line_height: Option<f32>,

        weight: Option<FontWeight>,

        style: Option<FontStyle>,
    },
}

impl FontRef {
    /// 引用的样式名。
    pub(crate) fn base(&self) -> Option<&str> {
        match self {
            Self::Named(name) => Some(name),
            Self::Inline { base, .. } => base.as_deref(),
        }
    }

    /// 在基础样式上套用这里的改动。
    pub(crate) fn apply(&self, base: FontSpec) -> FontSpec {
        let Self::Inline {
            size,
            line_height,
            weight,
            style,
            ..
        } = self
        else {
            return base;
        };
        let size = size.unwrap_or(base.size);
        FontSpec {
            size,
            line_height: line_height.unwrap_or(if base.size > 0.0 {
                base.line_height * size / base.size
            } else {
                size
            }),
            weight: weight.unwrap_or(base.weight),
            italic: style.map_or(base.italic, |style| style == FontStyle::Italic),
            family: base.family,
        }
    }
}
