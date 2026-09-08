//! 菜单栏里的「中 / 英」状态项。
//!
//! 输入源图标（Info.plist 的 tsInputMethodIconFileKey）没法动态换，所以自己放一个 NSStatusItem。
//! Caps Lock 的变化不会作为按键送到输入法，用一个定时器轮询系统状态刷新。
//! 状态项给了固定的 autosave 名：位置与可见性按这个名字存进偏好，用户 ⌘ 拖到输入法图标旁边后，
//! 切走再切回（隐藏再显示）仍在原位；不设名字系统按创建序号起名，隐藏后再显示会回到最左边。

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{NSMenu, NSStatusBar, NSStatusItem, NSVariableStatusItemLength};
use objc2_foundation::{NSObject, NSObjectProtocol, NSString, NSTimer, ns_string};

use crate::imk::modifiers;

/// 轮询 Caps Lock 状态的间隔。
const POLL_INTERVAL: f64 = 0.25;

pub struct ModeIndicator {
    /// 菜单栏状态项。
    item: Retained<NSStatusItem>,

    /// 轮询定时器；未激活时为 `None`。
    timer: Option<Retained<NSTimer>>,

    /// 上次显示的是否英文模式，避免每次轮询都重设标题。
    english: Option<bool>,

    /// 云联想开着：标题带云朵，让用户一眼知道上下文会发出去。
    cloud: bool,

    mtm: MainThreadMarker,
}

impl ModeIndicator {
    pub fn new(mtm: MainThreadMarker) -> Self {
        let item = NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
        item.setAutosaveName(Some(ns_string!("QingjianModeIndicator")));
        item.setVisible(false);
        Self {
            item,
            timer: None,
            english: None,
            cloud: false,
            mtm,
        }
    }

    /// 输入法激活：显示状态项并开始轮询。
    pub fn activate(&mut self) {
        self.english = None;
        self.update();
        self.item.setVisible(true);
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

    /// 输入法停用：隐藏状态项并停止轮询。
    pub fn deactivate(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.invalidate();
        }
        self.item.setVisible(false);
    }

    /// 点状态项弹出的菜单。
    pub fn set_menu(&self, menu: &NSMenu) {
        self.item.setMenu(Some(menu));
    }

    pub fn set_cloud(&mut self, cloud: bool) {
        self.cloud = cloud;
        self.english = None;
    }

    /// 按当前 Caps Lock 状态刷新标题。
    pub fn update(&mut self) {
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
    }

    unsafe impl NSObjectProtocol for ModeMonitor {}
);

impl ModeMonitor {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
