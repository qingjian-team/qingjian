//! 主线程定时器回调，只负责把轮询交回 Host。

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_foundation::{NSObject, NSObjectProtocol};

define_class!(
    // SAFETY: NSObject 没有子类化要求；没有实现 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    pub(super) struct LiveTicker;

    impl LiveTicker {
        #[unsafe(method(tick:))]
        fn tick(&self, _timer: Option<&AnyObject>) {
            crate::host::with(|h| h.poll_live_translation());
        }
    }

    unsafe impl NSObjectProtocol for LiveTicker {}
);

impl LiveTicker {
    pub(super) fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
