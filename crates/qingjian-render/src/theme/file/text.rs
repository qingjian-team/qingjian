//! 文字设置：缺省字族、命名的文字样式（节点用 `"font": "名字"` 引用）与覆盖率 gamma。

use std::collections::HashMap;

use bamboo_slip::{FamilyList, FontWeight};
use serde::Deserialize;

use super::FontStyle;
use super::adaptive::Adaptive;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct TextSettings {
    /// 文字抗锯齿覆盖率的 gamma：小于 1 笔画显粗，见 `docs/design/rendering.md`。
    pub(crate) gamma: Adaptive<f32>,

    /// 没写字族的样式用的字族；不写为界面字体。
    #[serde(default)]
    pub(crate) family: Option<FamilyList>,

    /// 命名的文字样式。
    pub(crate) styles: HashMap<String, StyleSpec>,
}

/// 一个命名文字样式（点）。
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct StyleSpec {
    pub(crate) size: f32,

    /// 行高：一行文字占的高度，字形在其中垂直居中。
    pub(crate) line_height: f32,

    #[serde(default)]
    pub(crate) weight: FontWeight,

    /// 不写用 `text.family`。
    #[serde(default)]
    pub(crate) family: Option<FamilyList>,

    #[serde(default)]
    pub(crate) style: FontStyle,
}

#[cfg(test)]
mod tests {
    use super::{StyleSpec, TextSettings};
    use crate::theme::file::FontStyle;

    #[test]
    fn a_style_falls_back_to_the_text_family_and_upright() {
        let text: TextSettings = serde_json::from_str(
            r#"{"gamma": 1.4, "styles": {"body": {"size": 16, "line_height": 20}}}"#,
        )
        .unwrap();
        assert_eq!(text.gamma.get(false), 1.4);
        assert!(text.family.is_none());
        let body = &text.styles["body"];
        assert_eq!((body.size, body.line_height), (16.0, 20.0));
        assert!(body.family.is_none());
        assert_eq!(body.style, FontStyle::Normal);
    }

    #[test]
    fn a_style_can_slant_and_carry_its_own_family() {
        let text: TextSettings = serde_json::from_str(
            r#"{"gamma": {"light": 1.0, "dark": 1.6},
                "styles": {"cap": {"size": 20, "line_height": 24, "style": "italic"}}}"#,
        )
        .unwrap();
        assert_eq!(text.gamma.get(false), 1.0);
        assert_eq!(text.gamma.get(true), 1.6);
        assert_eq!(text.styles["cap"].style, FontStyle::Italic);
    }

    #[test]
    fn gamma_and_styles_are_required() {
        assert!(serde_json::from_str::<TextSettings>(r#"{"styles": {}}"#).is_err());
        assert!(serde_json::from_str::<TextSettings>(r#"{"gamma": 1.0}"#).is_err());
        assert!(serde_json::from_str::<StyleSpec>(r#"{"size": 16}"#).is_err());
    }
}
