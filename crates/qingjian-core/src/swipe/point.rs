//! 轨迹上的一点，坐标是壳那边的像素（与键位同一个坐标系就行）。

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct SwipePoint {
    pub x: f32,

    pub y: f32,
}
