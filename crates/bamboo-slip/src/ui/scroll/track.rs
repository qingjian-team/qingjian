//! 滚动条的轨道：拖滑块按比例滚，点轨道跳过去并接着能拖。拖动的状态归交互层，使用方不用处理消息。

use crate::ui::Key;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollbarTrack {
    /// 它滚的那个滚动区（视口元素的 key）。
    pub scroller: Key,

    /// 沿横向滚（滑块左右走）。
    pub horizontal: bool,

    /// 滑块两头离视口边的距离（像素）。
    pub inset: f32,

    /// 滑块的长度（像素）。
    pub thumb: f32,
}

impl ScrollbarTrack {
    /// 滑块能走的距离（像素）。
    fn travel(&self, viewport: f32) -> f32 {
        (viewport - self.inset * 2.0 - self.thumb).max(0.0)
    }

    /// 滚动位置为 `offset`（最远 `max`）时，滑块起点离视口起点多远（像素）。
    pub(crate) fn thumb_start(&self, offset: f32, max: f32, viewport: f32) -> f32 {
        let fraction = if max > 0.0 {
            (offset / max).clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.inset + self.travel(viewport) * fraction
    }

    /// 指针离视口起点 `along`、抓在滑块的 `grab` 处时，该滚到哪。
    pub(crate) fn offset_at(&self, along: f32, grab: f32, max: f32, viewport: f32) -> f32 {
        let travel = self.travel(viewport);
        if travel <= 0.0 {
            return 0.0;
        }
        ((along - grab - self.inset) / travel).clamp(0.0, 1.0) * max
    }
}
