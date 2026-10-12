//! 一个盒子怎么画：填充、内侧边框、圆角（像素）。

use super::fill::Fill;
use crate::color::Color;

#[derive(Debug, Clone)]
pub(crate) struct BoxPaint {
    pub(crate) fill: Option<Fill>,

    /// 内侧边框：宽度（像素）与颜色。
    pub(crate) border: Option<(f32, Color)>,

    pub(crate) radius: f32,
}
