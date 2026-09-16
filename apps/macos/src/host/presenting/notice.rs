//! 候选窗口里一闪而过的提示（「没有选中的文字」这类），几秒后自动收起，敲任何键也收起。

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_foundation::{NSObject, NSObjectProtocol, NSTimer};

/// 提示显示多久。
const NOTICE_SECONDS: f64 = 2.5;

/// 一条正在显示的提示：到点自动收。
pub struct Notice {
    /// 收起用的定时器。
    timer: Retained<NSTimer>,
}

impl Notice {
    /// 起一个一次性定时器，到点让 Host 收提示。
    pub fn schedule(mtm: MainThreadMarker) -> Self {
        let target = NoticeTicker::new(mtm);
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                NOTICE_SECONDS,
                &target,
                sel!(tick:),
                None,
                false,
            )
        };
        Self { timer }
    }
}

impl Drop for Notice {
    fn drop(&mut self) {
        self.timer.invalidate();
    }
}

define_class!(
    // SAFETY: NSObject 没有子类化要求；没有实现 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    struct NoticeTicker;

    impl NoticeTicker {
        #[unsafe(method(tick:))]
        fn tick(&self, _timer: Option<&AnyObject>) {
            crate::host::with(|h| h.clear_notice());
        }
    }

    unsafe impl NSObjectProtocol for NoticeTicker {}
);

impl NoticeTicker {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
