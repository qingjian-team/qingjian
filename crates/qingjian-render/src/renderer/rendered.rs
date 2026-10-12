//! 渲染结果：位图加内容区（阴影边之外的那块）的位置。

use tiny_skia::Pixmap;

use super::{HitRegion, HitTarget};

pub struct Rendered {
    /// 预乘 RGBA 位图，含阴影边。
    pub pixmap: Pixmap,

    /// 内容区左上角在位图里的像素坐标。
    pub content_x: u32,

    pub content_y: u32,

    /// 内容区像素宽高（窗口该有的大小）。
    pub content_width: u32,

    pub content_height: u32,

    /// 渲染用的倍数，壳把像素换回点用。
    pub scale: f32,

    /// 还有动画在播：隔这么久调 [`crate::Renderer::tick`] 要下一帧；`None` 表示画完了，不用再要。
    pub next_frame: Option<std::time::Duration>,

    /// 候选窗口里可点的区域，译词在前（先命中）；状态条为空，它按 [`crate::RenderedStatus::cell_edges`] 分格。
    pub hits: Vec<HitRegion>,
}

impl Rendered {
    /// 内容区里一点（点，左上为原点）点中了什么。
    pub fn hit_at(&self, x: f32, y: f32) -> Option<HitTarget> {
        let (x, y) = (x * self.scale, y * self.scale);
        self.hits
            .iter()
            .find(|region| region.contains(x, y))
            .map(|region| region.target)
    }

    /// 内容区宽高换回点。
    pub fn content_size_points(&self) -> (f32, f32) {
        (
            self.content_width as f32 / self.scale,
            self.content_height as f32 / self.scale,
        )
    }
}
