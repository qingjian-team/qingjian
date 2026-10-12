//! 设置里的字号：盖过主题 `candidate`、`annotation` 两个文字样式的字号，行高按同一比例缩放。
//! 序号（`index`）、拼音（`preedit`）与表格行高跟着候选字缩放，同一行才对得齐；主题自定义的其它样式不动。

use super::font::FontSpec;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TextSizes {
    /// 候选字字号（点）；`None` 用主题的。
    pub candidate: Option<f32>,

    /// 译文字号（点）；`None` 用主题的。
    pub annotation: Option<f32>,
}

impl TextSizes {
    /// 配置里的数：不是正数的当没填。
    pub fn new(candidate: f32, annotation: f32) -> Self {
        let positive = |size: f32| (size.is_finite() && size > 0.0).then_some(size);
        Self {
            candidate: positive(candidate),
            annotation: positive(annotation),
        }
    }

    /// `name` 样式相对主题的缩放比；`theme_size` 是主题里对应基准样式（候选或译文）的字号。
    pub(super) fn ratio(&self, name: &str, theme_size: impl Fn(&str) -> Option<f32>) -> f32 {
        let (wanted, base) = match name {
            "candidate" | "index" | "preedit" => (self.candidate, "candidate"),
            "annotation" => (self.annotation, "annotation"),
            _ => return 1.0,
        };
        match (wanted, theme_size(base)) {
            (Some(wanted), Some(base)) if base > 0.0 => wanted / base,
            _ => 1.0,
        }
    }
}

/// 字号与行高一起乘。
pub(super) fn scaled(spec: FontSpec, ratio: f32) -> FontSpec {
    FontSpec {
        size: spec.size * ratio,
        line_height: spec.line_height * ratio,
        ..spec
    }
}
