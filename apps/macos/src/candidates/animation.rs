//! 候选窗动画的定时器：渲染器说还有动画在播（`Rendered::next_frame`）就按它给的间隔起，每跳要一帧新位图；播完或窗口收起就停，平时不占 CPU。

use std::time::Duration;

use objc2::rc::{Retained, Weak};
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_foundation::{NSObject, NSObjectProtocol, NSTimer};

use super::view::CandidateView;

#[derive(Default)]
pub struct AnimationTimer {
    /// 在跳的定时器与它的间隔；没有动画时为 `None`。
    timer: Option<(Retained<NSTimer>, Duration)>,
}

impl AnimationTimer {
    /// 按 `interval` 跳（渲染器给的：过渡 16 ms、循环动画 33 ms）；已按同样间隔在跳就不动，间隔变了重起。
    /// 每跳调 `view` 的 [`CandidateView::animation_frame`]。
    pub fn start(&mut self, mtm: MainThreadMarker, view: &CandidateView, interval: Duration) {
        if self
            .timer
            .as_ref()
            .is_some_and(|(_, running)| *running == interval)
        {
            return;
        }
        self.stop();
        let target = AnimationTicker::new(mtm, view);
        // SAFETY: 选择子 `tick:` 由 AnimationTicker 实现，签名是定时器回调要的 `(NSTimer)`。
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                interval.as_secs_f64(),
                &target,
                sel!(tick:),
                None,
                true,
            )
        };
        self.timer = Some((timer, interval));
    }

    pub fn stop(&mut self) {
        if let Some((timer, _)) = self.timer.take() {
            timer.invalidate();
        }
    }
}

/// 定时器的目标：弱引用视图，视图没了就什么也不做。
pub struct TickerIvars {
    view: Weak<CandidateView>,
}

define_class!(
    // SAFETY: NSObject 没有子类化要求；没有实现 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = TickerIvars]
    struct AnimationTicker;

    impl AnimationTicker {
        #[unsafe(method(tick:))]
        fn tick(&self, _timer: Option<&AnyObject>) {
            if let Some(view) = self.ivars().view.load() {
                view.animation_frame();
            }
        }
    }

    unsafe impl NSObjectProtocol for AnimationTicker {}
);

impl AnimationTicker {
    fn new(mtm: MainThreadMarker, view: &CandidateView) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(TickerIvars {
            view: Weak::from(view),
        });
        // SAFETY: NSObject 的 init
        unsafe { msg_send![super(this), init] }
    }
}
