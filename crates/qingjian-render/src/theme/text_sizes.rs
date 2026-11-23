//! 设置里的字号：盖过主题 `candidate`、`annotation` 两个文字样式的字号，行高按同一比例缩放。
//! 序号（`index`）、拼音（`preedit`）与表格行高跟着候选字缩放，同一行才对得齐；主题自定义的其它样式不动。

use bamboo_slip::FontSpec;

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

#[cfg(test)]
mod tests {
    use super::{TextSizes, scaled};
    use bamboo_slip::FontSpec;

    fn theme_size(name: &str) -> Option<f32> {
        match name {
            "candidate" => Some(16.0),
            "annotation" => Some(12.0),
            _ => None,
        }
    }

    #[test]
    fn non_positive_or_non_finite_sizes_count_as_unset() {
        assert_eq!(
            TextSizes::new(0.0, -3.0),
            TextSizes {
                candidate: None,
                annotation: None,
            }
        );
        assert_eq!(TextSizes::new(f32::NAN, 12.0).candidate, None);
        assert_eq!(TextSizes::new(f32::INFINITY, 12.0).candidate, None);
        assert_eq!(
            TextSizes::new(18.0, 14.0),
            TextSizes {
                candidate: Some(18.0),
                annotation: Some(14.0),
            }
        );
    }

    #[test]
    fn the_candidate_size_carries_the_index_and_preedit_along() {
        let sizes = TextSizes::new(20.0, 0.0);
        assert_eq!(sizes.ratio("candidate", theme_size), 1.25);
        assert_eq!(sizes.ratio("index", theme_size), 1.25);
        assert_eq!(sizes.ratio("preedit", theme_size), 1.25);
        // 没填的译文与不相干的样式都按原样
        assert_eq!(sizes.ratio("annotation", theme_size), 1.0);
        assert_eq!(sizes.ratio("anything-else", theme_size), 1.0);
    }

    #[test]
    fn the_annotation_size_has_its_own_base() {
        let sizes = TextSizes::new(0.0, 18.0);
        assert_eq!(sizes.ratio("annotation", theme_size), 1.5);
        assert_eq!(sizes.ratio("candidate", theme_size), 1.0);
    }

    #[test]
    fn a_missing_or_zero_base_leaves_the_ratio_alone() {
        let sizes = TextSizes::new(20.0, 20.0);
        assert_eq!(sizes.ratio("candidate", |_| None), 1.0);
        assert_eq!(sizes.ratio("candidate", |_| Some(0.0)), 1.0);
        assert_eq!(sizes.ratio("candidate", |_| Some(-4.0)), 1.0);
        // 默认的 TextSizes 什么都不改
        assert_eq!(TextSizes::default().ratio("candidate", theme_size), 1.0);
    }

    #[test]
    fn scaling_moves_the_size_and_the_line_height_together() {
        let spec = FontSpec::new(16.0, 20.0);
        let out = scaled(spec, 1.25);
        assert_eq!(out.size, 20.0);
        assert_eq!(out.line_height, 25.0);
        let half = scaled(spec, 0.5);
        assert_eq!(half.size, 8.0);
        assert_eq!(half.line_height, 10.0);
    }
}
