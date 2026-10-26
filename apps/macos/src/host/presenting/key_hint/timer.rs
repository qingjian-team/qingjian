//! 停顿计时：敲键后这么久没再敲，就当用户卡住了，让 Host 弹出键位提示。每敲一键换一个新的。

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_foundation::{NSObject, NSObjectProtocol, NSTimer};

/// 停多久算卡住。熟练的人两键之间远短于这个，平时看不到键盘。
const HESITATION_SECONDS: f64 = 0.7;

pub struct HesitationTimer {
    timer: Retained<NSTimer>,
}

impl HesitationTimer {
    pub fn schedule(mtm: MainThreadMarker) -> Self {
        let target = HesitationTicker::new(mtm);
        // SAFETY: 选择子 `tick:` 由 HesitationTicker 实现，签名是定时器回调要的 `(NSTimer)`。
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                HESITATION_SECONDS,
                &target,
                sel!(tick:),
                None,
                false,
            )
        };
        Self { timer }
    }
}

impl Drop for HesitationTimer {
    fn drop(&mut self) {
        self.timer.invalidate();
    }
}

define_class!(
    // SAFETY: NSObject 没有子类化要求；没有实现 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    struct HesitationTicker;

    impl HesitationTicker {
        #[unsafe(method(tick:))]
        fn tick(&self, _timer: Option<&AnyObject>) {
            crate::host::with(|h| h.key_hint_hesitated());
        }
    }

    unsafe impl NSObjectProtocol for HesitationTicker {}
);

impl HesitationTicker {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        // SAFETY: NSObject 的 init
        unsafe { msg_send![super(this), init] }
    }
}
