//! 一个滚动区上一帧的大小：视口与内容的宽高（像素）。

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollExtent {
    pub viewport: (f32, f32),

    pub content: (f32, f32),
}

impl ScrollExtent {
    /// 最多能滚多远。
    pub fn max(&self) -> (f32, f32) {
        (
            (self.content.0 - self.viewport.0).max(0.0),
            (self.content.1 - self.viewport.1).max(0.0),
        )
    }
}
