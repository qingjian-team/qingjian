//! 节点在窗口里的位置、大小（像素，相对根节点左上角）、不透明度与颜色：过渡的起点、终点与中间值。

use crate::color::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub x: f32,

    pub y: f32,

    pub width: f32,

    pub height: f32,

    pub opacity: f32,

    /// 节点的主色（盒子的纯色填充、文字或图标的颜色，见 [`crate::Visual::color`]）；没有填充的盒子为 `None`，
    /// 与有颜色的一端过渡时当作同色全透明。
    pub color: Option<Color>,
}

impl Placement {
    /// 从 `self` 到 `to` 走了 `progress`（可略超出 0–1，回弹曲线）。
    pub fn lerp(self, to: Self, progress: f32) -> Self {
        let mix = |a: f32, b: f32| a + (b - a) * progress;
        Self {
            x: mix(self.x, to.x),
            y: mix(self.y, to.y),
            width: mix(self.width, to.width).max(0.0),
            height: mix(self.height, to.height).max(0.0),
            opacity: mix(self.opacity, to.opacity).clamp(0.0, 1.0),
            color: match (self.color, to.color) {
                (Some(from), Some(to)) => Some(from.lerp(to, progress)),
                (None, Some(to)) => Some(to.transparent().lerp(to, progress)),
                (Some(from), None) => Some(from.lerp(from.transparent(), progress)),
                (None, None) => None,
            },
        }
    }
}
