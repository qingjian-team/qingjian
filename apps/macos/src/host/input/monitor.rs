//! 常驻所选输入源通知；焦点停用期间仍观察 ABC 等输入源的切换。

use crate::app::input_source;
use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_foundation::{
    NSDistributedNotificationCenter, NSNotification, NSNotificationSuspensionBehavior, NSObject,
    NSObjectProtocol,
};

// SAFETY: 只在主线程创建，通知使用本类声明的选择器，不捕获客户端。
define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    pub(crate) struct SourceMonitor;

    impl SourceMonitor {
        #[unsafe(method(sourceChanged:))]
        fn source_changed(&self, _notification: Option<&NSNotification>) {
            crate::host::with(|h| h.sync_input_source());
        }
    }

    unsafe impl NSObjectProtocol for SourceMonitor {}
);

impl SourceMonitor {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        let this: Retained<Self> = unsafe { msg_send![super(this), init] };
        unsafe {
            NSDistributedNotificationCenter::defaultCenter()
                .addObserver_selector_name_object_suspensionBehavior(
                    &this,
                    sel!(sourceChanged:),
                    Some(input_source::selected_source_notification()),
                    None,
                    NSNotificationSuspensionBehavior::DeliverImmediately,
                );
        }
        this
    }
}
