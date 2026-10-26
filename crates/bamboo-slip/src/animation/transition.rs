//! 一个在播的过渡：哪个节点、从哪到哪、什么时候开始、多久、什么曲线。

use std::time::{Duration, Instant};

use taffy::NodeId;

use super::{Easing, Placement};

#[derive(Debug, Clone)]
pub struct Transition {
    /// 场景树里的节点（这一帧的）。
    pub node: NodeId,

    /// 配对用的 `id`（重复时带序号）。
    pub key: String,

    pub from: Placement,

    pub to: Placement,

    pub start: Instant,

    pub duration: Duration,

    pub easing: Easing,
}

impl Transition {
    /// `now` 时刻的位置。
    pub fn at(&self, now: Instant) -> Placement {
        self.from
            .lerp(self.to, self.easing.apply(self.progress(now)))
    }

    /// 时间进度 0–1。
    fn progress(&self, now: Instant) -> f32 {
        if self.duration.is_zero() {
            return 1.0;
        }
        (now.saturating_duration_since(self.start).as_secs_f32() / self.duration.as_secs_f32())
            .min(1.0)
    }

    pub fn finished(&self, now: Instant) -> bool {
        self.progress(now) >= 1.0
    }
}
