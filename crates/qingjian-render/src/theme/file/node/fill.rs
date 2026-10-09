//! 框的填充：纯色（颜色写法同 `color`，可带条件）、线性 / 径向渐变、图片（拉伸或九宫格）。
//!
//! - 线性：`{ "linear": 90, "stops": ["@a", "@b"] }`，角度同 CSS（0 自下而上、90 自左而右），渐变线长到盒子对角。
//! - 径向：`{ "radial": [0.5, 0.5], "stops": [...] }`，圆心按盒子宽高的比例，半径到最远的角。
//! - 色标：颜色（均分），或 `["@a", 0.3]` 带位置（0–1）。
//! - 图片：`{ "image": "images/bg.png", "slice": [24, 24, 24, 24], "scale": 2 }`，路径相对 `theme.json`；
//!   `slice` 是九宫格四边切进去的图片像素（上右下左），不写就整张拉伸；`scale` 是一个点对几个图片像素（2 倍图写 2），缺省 1。

use serde::Deserialize;

use crate::theme::file::ColorSpec;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub(crate) enum FillSpec {
    Linear {
        /// 角度（度）。
        linear: f32,

        stops: Vec<StopSpec>,
    },

    Radial {
        /// 圆心，盒子宽高的比例。
        radial: [f32; 2],

        stops: Vec<StopSpec>,
    },

    Image {
        /// 相对 `theme.json` 的路径。
        image: String,

        /// 九宫格切边（图片像素，上右下左）。
        slice: Option<[f32; 4]>,

        /// 一个点对几个图片像素。
        #[serde(default = "default_scale")]
        scale: f32,
    },

    Color(ColorSpec),
}

/// 渐变的一个色标。
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub(crate) enum StopSpec {
    /// 带位置（0–1）。
    At(ColorSpec, f32),

    /// 不带位置，按顺序均分。
    Even(ColorSpec),
}

fn default_scale() -> f32 {
    1.0
}

impl FillSpec {
    /// 用到的全部颜色（校验用）。
    pub(crate) fn colors(&self) -> Vec<&ColorSpec> {
        match self {
            Self::Linear { stops, .. } | Self::Radial { stops, .. } => {
                stops.iter().map(StopSpec::color).collect()
            }
            Self::Image { .. } => Vec::new(),
            Self::Color(color) => vec![color],
        }
    }
}

impl StopSpec {
    pub(crate) fn color(&self) -> &ColorSpec {
        match self {
            Self::At(color, _) | Self::Even(color) => color,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_fill_form() {
        let parse = |json: &str| serde_json::from_str::<FillSpec>(json).unwrap();
        assert!(matches!(parse(r#""@accent""#), FillSpec::Color(_)));
        assert!(matches!(
            parse(r#"{ "if": "highlighted", "then": "@a", "else": "@b" }"#),
            FillSpec::Color(ColorSpec::Switch { .. })
        ));
        let FillSpec::Linear { linear, stops } =
            parse(r##"{ "linear": 90, "stops": ["#ff0000", ["@b", 0.3]] }"##)
        else {
            panic!("not linear");
        };
        assert_eq!(linear, 90.0);
        assert!(matches!(stops[1], StopSpec::At(_, position) if position == 0.3));
        assert!(matches!(
            parse(r#"{ "radial": [0.5, 0.2], "stops": ["@a", "@b"] }"#),
            FillSpec::Radial { .. }
        ));
        let FillSpec::Image { slice, scale, .. } =
            parse(r#"{ "image": "images/bg.png", "slice": [8, 8, 8, 8] }"#)
        else {
            panic!("not image");
        };
        assert_eq!((slice, scale), (Some([8.0; 4]), 1.0));
    }
}
