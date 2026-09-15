//! 设置窗口的 NSWindow 子类：系统设置式的透明标题栏，关窗时把进程切回纯后台。

use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSWindow, NSWindowStyleMask,
    NSWindowTitleVisibility,
};
use objc2_foundation::{NSObjectProtocol, NSRect};

define_class!(
    // SAFETY: NSWindow 允许子类化；没有实现 Drop。
    #[unsafe(super(NSWindow))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    /// 设置窗口的 NSWindow：关窗时把进程的激活策略切回 Prohibited，让输入法回到纯后台。
    pub struct PreferencesPanel;

    impl PreferencesPanel {
        #[unsafe(method(close))]
        fn close(&self) {
            let mtm = MainThreadMarker::from(self);
            NSApplication::sharedApplication(mtm)
                .setActivationPolicy(NSApplicationActivationPolicy::Prohibited);
            let _: () = unsafe { msg_send![super(self), close] };
        }
    }

    unsafe impl NSObjectProtocol for PreferencesPanel {}
);

impl PreferencesPanel {
    pub fn new(mtm: MainThreadMarker, content: NSRect) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        let this: Retained<Self> = unsafe {
            msg_send![
                super(this),
                initWithContentRect: content,
                styleMask: NSWindowStyleMask::Titled
                    | NSWindowStyleMask::Closable
                    | NSWindowStyleMask::FullSizeContentView,
                backing: NSBackingStoreType::Buffered,
                defer: false,
            ]
        };
        // 程序建的 NSWindow 默认关窗即释放，我们还握着 Retained，必须关掉
        unsafe { this.setReleasedWhenClosed(false) };
        // 系统设置式外观：标题栏透明、不显示窗口标题，侧栏一直伸到窗口顶；页名由内容列自己画
        this.setTitlebarAppearsTransparent(true);
        this.setTitleVisibility(NSWindowTitleVisibility::Hidden);
        this.setMovableByWindowBackground(true);
        this
    }

    /// 切到 Accessory（有窗口、无 Dock 图标）并把窗口带到最前，文本框才拿得到键盘焦点。
    pub fn present(&self) {
        let mtm = MainThreadMarker::from(self);
        let app = NSApplication::sharedApplication(mtm);
        super::edit_menu::install(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
        #[allow(deprecated)]
        app.activateIgnoringOtherApps(true);
        self.makeKeyAndOrderFront(None);
    }
}
