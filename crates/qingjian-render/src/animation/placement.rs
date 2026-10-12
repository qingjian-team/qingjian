//! 节点在窗口里的位置、大小（像素，相对根节点左上角）与不透明度：过渡的起点、终点与中间值。

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Placement {
    pub(crate) x: f32,

    pub(crate) y: f32,

    pub(crate) width: f32,

    pub(crate) height: f32,

    pub(crate) opacity: f32,
}

impl Placement {
    /// 从 `self` 到 `to` 走了 `progress`（可略超出 0–1，回弹曲线）。
    pub(crate) fn lerp(self, to: Self, progress: f32) -> Self {
        let mix = |a: f32, b: f32| a + (b - a) * progress;
        Self {
            x: mix(self.x, to.x),
            y: mix(self.y, to.y),
            width: mix(self.width, to.width).max(0.0),
            height: mix(self.height, to.height).max(0.0),
            opacity: mix(self.opacity, to.opacity).clamp(0.0, 1.0),
        }
    }
}
