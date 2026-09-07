//! 联想结果的轮询定时器。发出请求时开始轮询，结果到了或超时就停，平时不占 CPU。

use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_foundation::{NSObject, NSObjectProtocol, NSTimer};

/// 轮询间隔。
const POLL_INTERVAL: f64 = 0.05;

/// 最长轮询多久；请求本身有超时，这里只是兜底。
const MAX_WAIT: Duration = Duration::from_secs(12);

pub struct PredictMonitor {
    /// 定时器；没在等结果时为 `None`。
    timer: Option<Retained<NSTimer>>,

    /// 本轮开始等待的时间。
    since: Option<Instant>,

    mtm: MainThreadMarker,
}

impl PredictMonitor {
    pub fn new(mtm: MainThreadMarker) -> Self {
        Self {
            timer: None,
            since: None,
            mtm,
        }
    }

    /// 有请求在飞：开始（或继续）轮询。
    pub fn start(&mut self) {
        self.since = Some(Instant::now());
        if self.timer.is_some() {
            return;
        }
        let target = PredictTicker::new(self.mtm);
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
        self.since = None;
    }

    /// 等太久了就放弃。
    pub fn expired(&self) -> bool {
        self.since.is_some_and(|since| since.elapsed() > MAX_WAIT)
    }
}

define_class!(
    // SAFETY: NSObject 没有子类化要求；没有实现 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    struct PredictTicker;

    impl PredictTicker {
        #[unsafe(method(tick:))]
        fn tick(&self, _timer: Option<&AnyObject>) {
            crate::host::with(|h| h.poll_prediction());
        }
    }

    unsafe impl NSObjectProtocol for PredictTicker {}
);

impl PredictTicker {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
