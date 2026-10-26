//! 建一帧要用的东西：设置状态、控件主题、配色与缩放。

use bamboo_slip::WidgetTheme;
use bamboo_slip::taffy::{LengthPercentage, Size};

use super::palette::Palette;
use crate::settings::Settings;

/// 建一帧要用的东西。
pub struct Page<'a> {
    pub settings: &'a Settings,

    pub theme: WidgetTheme,

    pub palette: &'a Palette,

    pub scale: f32,

    /// 右边内容区扣掉左右边距后的宽度（像素）。
    pub content_width: f32,
}

impl Page<'_> {
    pub fn px(&self, points: f32) -> f32 {
        points * self.scale
    }

    pub fn gap(&self, points: f32) -> Size<LengthPercentage> {
        let value = LengthPercentage::length(self.px(points));
        Size {
            width: value,
            height: value,
        }
    }
}
