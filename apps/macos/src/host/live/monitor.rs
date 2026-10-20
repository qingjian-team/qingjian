//! 本地实时翻译结果的独立轮询定时器。

use super::ticker::LiveTicker;
use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_foundation::NSTimer;

/// 轮询间隔。
const POLL_INTERVAL: f64 = 0.05;

pub struct LiveMonitor {
    /// 定时器；没在等结果时为 `None`。
    timer: Option<Retained<NSTimer>>,

    mtm: MainThreadMarker,
}

impl LiveMonitor {
    pub fn new(mtm: MainThreadMarker) -> Self {
        Self { timer: None, mtm }
    }

    /// 有请求在飞：开始（或继续）轮询。
    pub fn start(&mut self) {
        if self.timer.is_some() {
            return;
        }
        let target = LiveTicker::new(self.mtm);
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                POLL_INTERVAL,
                &target,
                sel!(tick:),
                None,
                true,
            )
        };
        self.timer = Some(timer);
    }

    pub fn stop(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.invalidate();
        }
    }
}
