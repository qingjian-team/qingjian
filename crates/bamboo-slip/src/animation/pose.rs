//! 循环动画某一刻的姿态：平移（像素）、旋转（度）、缩放、不透明度，作用于节点整棵子树。

use tiny_skia::Transform;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    pub x: f32,

    pub y: f32,

    pub rotate: f32,

    pub scale: f32,

    pub opacity: f32,
}

impl Pose {
    pub const REST: Self = Self {
        x: 0.0,
        y: 0.0,
        rotate: 0.0,
        scale: 1.0,
        opacity: 1.0,
    };

    /// 绕 `(cx, cy)` 的变换：先缩放、旋转，再平移。
    pub fn transform(&self, cx: f32, cy: f32) -> Transform {
        Transform::from_translate(-cx, -cy)
            .post_scale(self.scale, self.scale)
            .post_rotate(self.rotate)
            .post_translate(cx + self.x, cy + self.y)
    }
}
