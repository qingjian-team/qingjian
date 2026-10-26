//! 一个字母键的几何，坐标是壳那边的像素（与轨迹同一个坐标系就行）。

use serde::Deserialize;

use super::SwipePoint;

/// 一个字母键：中心与大小。
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct SwipeKey {
    pub letter: char,

    /// 中心。
    pub x: f32,

    pub y: f32,

    pub width: f32,

    pub height: f32,
}

impl SwipeKey {
    /// 点到这个键中心的距离，横竖按键的宽高归一（键不是正方形时，竖着偏一点不比横着偏一点更远）。
    pub fn distance(&self, point: SwipePoint) -> f32 {
        let dx = (point.x - self.x) / self.width.max(1.0);
        let dy = (point.y - self.y) / self.height.max(1.0);
        (dx * dx + dy * dy).sqrt()
    }
}
