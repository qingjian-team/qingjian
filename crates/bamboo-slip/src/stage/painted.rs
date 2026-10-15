//! 画出来的一帧：位图加内容区（阴影边之外的那块）的位置。

use tiny_skia::Pixmap;

#[derive(Debug, Clone)]
pub struct Painted {
    /// 预乘 RGBA 位图，含阴影边。
    pub pixmap: Pixmap,

    /// 内容区左上角在位图里的像素坐标。
    pub content_x: u32,

    pub content_y: u32,

    /// 内容区像素宽高（窗口该有的大小）。
    pub content_width: u32,

    pub content_height: u32,

    /// 画这一帧用的倍数，使用方把像素换回点用。
    pub scale: f32,

    /// 还有动画在播：隔这么久调 [`crate::Stage::tick`] 要下一帧；`None` 表示画完了。
    pub next_frame: Option<std::time::Duration>,
}

impl Painted {
    /// 内容区宽高换回点。
    pub fn content_size_points(&self) -> (f32, f32) {
        (
            self.content_width as f32 / self.scale,
            self.content_height as f32 / self.scale,
        )
    }
}
