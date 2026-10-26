//! 布局之后的一个节点：在根节点坐标里的盒子，以及上级裁剪留给它的可见范围。

use taffy::NodeId;

/// 左、上、宽、高（像素）。
pub type Rect = (f32, f32, f32, f32);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Located {
    pub node: NodeId,

    pub rect: Rect,

    /// 上级裁剪区的交集；`None` 为没有上级裁剪。
    pub clip: Option<Rect>,
}

impl Located {
    /// `(x, y)` 落在盒子里，并且没被上级裁掉。
    pub fn contains(&self, x: f32, y: f32) -> bool {
        inside(self.rect, x, y) && self.clip.is_none_or(|clip| inside(clip, x, y))
    }
}

fn inside((left, top, width, height): Rect, x: f32, y: f32) -> bool {
    x >= left && x < left + width && y >= top && y < top + height
}

/// 两个矩形的交集（可能宽高为 0）。
pub fn intersect(a: Rect, b: Rect) -> Rect {
    let left = a.0.max(b.0);
    let top = a.1.max(b.1);
    let right = (a.0 + a.2).min(b.0 + b.2);
    let bottom = (a.1 + a.3).min(b.1 + b.3);
    (left, top, (right - left).max(0.0), (bottom - top).max(0.0))
}
