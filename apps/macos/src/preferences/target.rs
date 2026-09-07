use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_foundation::{NSObject, NSObjectProtocol};

use crate::host;

define_class!(
    // SAFETY: NSObject 没有子类化要求；没有实现 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    /// 设置窗口所有控件的 target：一个 `changed:` 选择器，靠 tag 区分控件。
    pub struct PreferencesTarget;

    impl PreferencesTarget {
        #[unsafe(method(changed:))]
        fn changed(&self, sender: Option<&AnyObject>) {
            if let Some((setting, value)) = super::setting_from_sender(sender) {
                host::with(|h| h.change_setting(setting, value));
            }
        }
    }

    unsafe impl NSObjectProtocol for PreferencesTarget {}
);

impl PreferencesTarget {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
