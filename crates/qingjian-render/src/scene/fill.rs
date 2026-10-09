//! 盒子的填充：纯色、线性 / 径向渐变、图片（PNG 或 SVG）。颜色已按外观与数据解析好，长度已是像素。

use std::sync::Arc;

use tiny_skia::Pixmap;

use crate::color::Color;
use crate::theme::SvgImage;

#[derive(Debug, Clone)]
pub(crate) enum Fill {
    Solid(Color),

    /// 角度同 CSS（度），色标（颜色、0–1 位置）。
    Linear {
        angle: f32,
        stops: Vec<(Color, f32)>,
    },

    /// 圆心按盒子宽高的比例，半径到最远的角。
    Radial {
        center: [f32; 2],

        stops: Vec<(Color, f32)>,
    },

    /// 图片：`slice` 是九宫格切边（图片像素，上右下左），`None` 整张拉伸；`pixels_per_px` 是一个画布像素对几个图片像素。
    Image {
        pixmap: Arc<Pixmap>,

        slice: Option<[f32; 4]>,

        pixels_per_px: f32,
    },

    /// SVG：画的时候按盒子（拉伸）或原始尺寸 × `px_per_unit`（九宫格）栅格；`slice` 是 SVG 单位的切边。
    Svg {
        image: Arc<SvgImage>,

        slice: Option<[f32; 4]>,

        /// 一个 SVG 单位（点）对几个画布像素。
        px_per_unit: f32,
    },
}
