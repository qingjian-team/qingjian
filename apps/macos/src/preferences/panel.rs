use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSEvent, NSEventType, NSText,
    NSTextField, NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{NSObjectProtocol, NSRect};

define_class!(
    // SAFETY: NSWindow 允许子类化；没有实现 Drop。
    #[unsafe(super(NSWindow))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    /// 设置窗口的 NSWindow：关窗时把进程的激活策略切回 Prohibited，让输入法回到纯后台。
    /// 文本框只在「结束编辑」时发值，点别的控件、切走、关窗都不算结束，改了的值就悬着；这几处主动结束编辑，与按回车一样生效。
    pub struct PreferencesPanel;

    impl PreferencesPanel {
        #[unsafe(method(sendEvent:))]
        fn send_event(&self, event: &NSEvent) {
            if event.r#type() == NSEventType::LeftMouseDown && !self.click_in_editing_field(event) {
                self.end_editing();
            }
            let _: () = unsafe { msg_send![super(self), sendEvent: event] };
        }

        #[unsafe(method(resignKeyWindow))]
        fn resign_key_window(&self) {
            self.end_editing();
            let _: () = unsafe { msg_send![super(self), resignKeyWindow] };
        }

        #[unsafe(method(close))]
        fn close(&self) {
            self.end_editing();
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
                styleMask: NSWindowStyleMask::Titled | NSWindowStyleMask::Closable,
                backing: NSBackingStoreType::Buffered,
                defer: false,
            ]
        };
        // 程序建的 NSWindow 默认关窗即释放，我们还握着 Retained，必须关掉
        unsafe { this.setReleasedWhenClosed(false) };
        this
    }

    /// 正在编辑的文本框交出焦点，`sendsActionOnEndEditing` 让它把值发出去。
    fn end_editing(&self) {
        let editing = self.field_editor().is_some_and(|editor| {
            self.firstResponder().is_some_and(|responder| {
                Retained::as_ptr(&responder).cast::<AnyObject>()
                    == Retained::as_ptr(&editor).cast::<AnyObject>()
            })
        });
        if editing {
            self.makeFirstResponder(None);
        }
    }

    /// 窗口共用的字段编辑器（文本框编辑时真正收键的那个视图）；还没建过为 `None`。
    fn field_editor(&self) -> Option<Retained<NSText>> {
        // SAFETY: create 为 false 不新建，对象参数为空取窗口共用的那个
        unsafe { self.fieldEditor_forObject(false, None) }
    }

    /// 点的是正在编辑的那个文本框（继续编辑，不打断）。
    fn click_in_editing_field(&self, event: &NSEvent) -> bool {
        let Some(content) = self.contentView() else {
            return false;
        };
        let Some(hit) = content.hitTest(event.locationInWindow()) else {
            return false;
        };
        let Some(editor) = self.field_editor() else {
            return false;
        };
        if hit.isDescendantOf(&editor) {
            return true;
        }
        hit.downcast::<NSTextField>()
            .ok()
            .and_then(|field| field.currentEditor())
            .is_some_and(|current| {
                Retained::as_ptr(&current).cast::<AnyObject>()
                    == Retained::as_ptr(&editor).cast::<AnyObject>()
            })
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
