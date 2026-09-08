//! 菜单栏里的「中 / 英」状态项。
//!
//! 输入源图标（Info.plist 的 tsInputMethodIconFileKey）没法动态换，所以自己放一个 NSStatusItem。
//! Caps Lock 的变化不会作为按键送到输入法，用一个定时器轮询系统状态刷新。
//!
//! 状态项一旦创建就**不再隐藏**：`setVisible(false)` 再 `setVisible(true)` 会把它重新排到菜单栏最左边，用户 ⌘ 拖到输入法图标旁的位置就丢了
//! （固定 autosave 名也保不住），而焦点每进出一次输入框 IMK 就 deactivate / activate 一轮。
//! 停用时改成收成零宽、清空标题，并且延迟 [`COLLAPSE_DELAY`] 再收：焦点只是在输入框之间挪的话，半秒内就会再次激活，根本收不下去；
//! 真换到别的输入法才收起来，切回来再展开，位置一直在。

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{NSMenu, NSStatusBar, NSStatusItem, NSVariableStatusItemLength};
use objc2_foundation::{NSObject, NSObjectProtocol, NSString, NSTimer, ns_string};

use crate::imk::modifiers;

/// 轮询 Caps Lock 状态的间隔。
const POLL_INTERVAL: f64 = 0.25;

/// 停用后隔多久才把状态项收起：焦点在输入框之间挪动时 deactivate 与下一次 activate 只隔几十毫秒。
const COLLAPSE_DELAY: f64 = 0.5;

pub struct ModeIndicator {
    /// 菜单栏状态项。
    item: Retained<NSStatusItem>,

    /// 轮询定时器；未激活时为 `None`。
    timer: Option<Retained<NSTimer>>,

    /// 停用后延迟收起的一次性定时器；再次激活时取消。
    collapse_timer: Option<Retained<NSTimer>>,

    /// 正展开着（输入法激活中）。收起时不刷新标题。
    shown: bool,

    /// 上次显示的是否英文模式，避免每次轮询都重设标题。
    english: Option<bool>,

    /// 云联想开着：标题带云朵，让用户一眼知道上下文会发出去。
    cloud: bool,

    mtm: MainThreadMarker,
}

impl ModeIndicator {
    pub fn new(mtm: MainThreadMarker) -> Self {
        let item = NSStatusBar::systemStatusBar().statusItemWithLength(0.0);
        item.setAutosaveName(Some(ns_string!("QingjianModeIndicator")));
        item.setVisible(true);
        Self {
            item,
            timer: None,
            collapse_timer: None,
            shown: false,
            english: None,
            cloud: false,
            mtm,
        }
    }

    /// 输入法激活：展开状态项并开始轮询；停用时安排的收起取消。
    pub fn activate(&mut self) {
        if let Some(timer) = self.collapse_timer.take() {
            timer.invalidate();
        }
        if !self.shown {
            self.shown = true;
            self.item.setLength(NSVariableStatusItemLength);
        }
        self.english = None;
        self.update();
        if self.timer.is_none() {
            let target = ModeMonitor::new(self.mtm);
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
    }

    /// 输入法停用：停止轮询，半秒后没再激活就收起。
    pub fn deactivate(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.invalidate();
        }
        if self.collapse_timer.is_some() {
            return;
        }
        let target = ModeMonitor::new(self.mtm);
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                COLLAPSE_DELAY,
                &target,
                sel!(collapse:),
                None,
                false,
            )
        };
        self.collapse_timer = Some(timer);
    }

    /// 收成零宽、清空标题；位置保留。
    pub fn collapse(&mut self) {
        self.collapse_timer = None;
        if !self.shown {
            return;
        }
        self.shown = false;
        self.english = None;
        if let Some(button) = self.item.button(self.mtm) {
            button.setTitle(ns_string!(""));
        }
        self.item.setLength(0.0);
    }

    /// 点状态项弹出的菜单。
    pub fn set_menu(&self, menu: &NSMenu) {
        self.item.setMenu(Some(menu));
    }

    pub fn set_cloud(&mut self, cloud: bool) {
        self.cloud = cloud;
        self.english = None;
    }

    /// 按当前 Caps Lock 状态刷新标题；收起时不动。
    pub fn update(&mut self) {
        if !self.shown {
            return;
        }
        let english = modifiers::caps_lock_on();
        if self.english == Some(english) {
            return;
        }
        self.english = Some(english);
        if let Some(button) = self.item.button(self.mtm) {
            let mode = if english { "英" } else { "中" };
            let title = if self.cloud {
                format!("{mode} ☁︎")
            } else {
                mode.to_owned()
            };
            button.setTitle(&NSString::from_str(&title));
        }
    }
}

define_class!(
    // SAFETY: NSObject 没有子类化要求；没有实现 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    struct ModeMonitor;

    impl ModeMonitor {
        #[unsafe(method(tick:))]
        fn tick(&self, _timer: Option<&AnyObject>) {
            crate::host::with(|h| h.indicator.update());
        }

        #[unsafe(method(collapse:))]
        fn collapse(&self, _timer: Option<&AnyObject>) {
            crate::host::with(|h| h.indicator.collapse());
        }
    }

    unsafe impl NSObjectProtocol for ModeMonitor {}
);

impl ModeMonitor {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
