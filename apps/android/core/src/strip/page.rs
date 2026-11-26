//! 桌面候选主题中的翻页箭头点击区域。

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct StripPage {
    pub step: isize,

    pub x: f32,

    pub y: f32,

    pub width: f32,

    pub height: f32,
}
