//! 菜单栏图标与它的菜单。按仓库约定用系统原生控件，不走自绘；菜单每次打开前按当前状态与界面语言重建。

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject, Sel};
use objc2::{AnyThread, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSControlStateValueOff, NSControlStateValueOn, NSImage, NSMenu, NSMenuDelegate, NSMenuItem,
    NSStatusBar, NSStatusItem, NSVariableStatusItemLength,
};
use objc2_foundation::{NSData, NSObject, NSObjectProtocol, NSSize, NSString};

use super::{autostart, listener};
use crate::i18n::tr;

/// 菜单栏图标：模板图（只看透明度，系统按菜单栏深浅上色），36 像素见方，按 18 点显示。
/// 从托盘图标的 `assets/icon/windows/clipboard-tray.svg` 把白墨换成黑墨，`rsvg-convert -w 36 -h 36` 画出来。
const ICON: &[u8] = include_bytes!("../../../resources/menubar.png");

/// 图标显示多大（点）。
const ICON_POINTS: f64 = 18.0;

define_class!(
    // SAFETY: NSObject 没有子类化要求；没有实现 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    /// 菜单项的 target，也是菜单的代理：剪贴板进程平时没有 key window，nil target 走不到响应链。
    struct MenuTarget;

    impl MenuTarget {
        #[unsafe(method(openPanel:))]
        fn open_panel(&self, _sender: Option<&AnyObject>) {
            listener::open_from_menu(self.mtm());
        }

        #[unsafe(method(togglePause:))]
        fn toggle_pause(&self, _sender: Option<&AnyObject>) {
            listener::toggle_pause();
        }

        #[unsafe(method(openSettings:))]
        fn open_settings(&self, _sender: Option<&AnyObject>) {
            listener::open_settings();
        }

        #[unsafe(method(toggleAutostart:))]
        fn toggle_autostart(&self, _sender: Option<&AnyObject>) {
            autostart::set(!autostart::enabled());
        }

        #[unsafe(method(quit:))]
        fn quit(&self, _sender: Option<&AnyObject>) {
            listener::quit();
        }
    }

    unsafe impl NSObjectProtocol for MenuTarget {}

    unsafe impl NSMenuDelegate for MenuTarget {
        #[unsafe(method(menuNeedsUpdate:))]
        fn menu_needs_update(&self, menu: &NSMenu) {
            fill(self, menu);
        }
    }
);

impl MenuTarget {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        // SAFETY: NSObject 的 init。
        unsafe { msg_send![super(this), init] }
    }
}

/// 状态项、菜单与 target 都要一直留着，放掉了图标就没了。
struct StatusMenu {
    _item: Retained<NSStatusItem>,
    _menu: Retained<NSMenu>,
    _target: Retained<MenuTarget>,
}

thread_local! {
    static INSTALLED: RefCell<Option<StatusMenu>> = const { RefCell::new(None) };
}

fn item(
    mtm: MainThreadMarker,
    title: &str,
    action: Sel,
    target: &MenuTarget,
) -> Retained<NSMenuItem> {
    // SAFETY: 选择器都在 MenuTarget 上定义，签名 (id) -> void。
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            mtm.alloc(),
            &NSString::from_str(title),
            Some(action),
            &NSString::from_str(""),
        )
    };
    // SAFETY: target 由 StatusMenu 一直留着，比菜单项活得久。
    unsafe { item.setTarget(Some(target)) };
    item
}

fn set_checked(item: &NSMenuItem, on: bool) {
    item.setState(if on {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    });
}

/// 按现在的状态重建菜单：语言随配置换，暂停打勾。
fn fill(target: &MenuTarget, menu: &NSMenu) {
    let mtm = target.mtm();
    let state = listener::menu_state();
    menu.removeAllItems();
    menu.addItem(&item(mtm, tr("tray.open"), sel!(openPanel:), target));
    let pause = item(mtm, tr("tray.pause"), sel!(togglePause:), target);
    set_checked(&pause, state.paused);
    menu.addItem(&pause);
    if state.has_settings {
        menu.addItem(&item(mtm, tr("tray.settings"), sel!(openSettings:), target));
    }
    if state.standalone {
        let autostart = item(mtm, tr("tray.autostart"), sel!(toggleAutostart:), target);
        set_checked(&autostart, autostart::enabled());
        menu.addItem(&autostart);
    }
    menu.addItem(&NSMenuItem::separatorItem(mtm));
    menu.addItem(&item(mtm, tr("tray.quit"), sel!(quit:), target));
}

fn icon() -> Option<Retained<NSImage>> {
    let image = NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(ICON))?;
    image.setSize(NSSize::new(ICON_POINTS, ICON_POINTS));
    image.setTemplate(true);
    Some(image)
}

/// 装上菜单栏图标。只在主线程调，只装一次。
pub fn install(mtm: MainThreadMarker) {
    if INSTALLED.with_borrow(Option::is_some) {
        return;
    }
    let target = MenuTarget::new(mtm);
    let menu = NSMenu::new(mtm);
    menu.setDelegate(Some(ProtocolObject::from_ref(&*target)));
    fill(&target, &menu);
    let item = NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
    if let Some(button) = item.button(mtm) {
        match icon() {
            Some(image) => button.setImage(Some(&image)),
            None => button.setTitle(&NSString::from_str(tr("tray.tip"))),
        }
        button.setToolTip(Some(&NSString::from_str(tr("tray.tip"))));
    }
    item.setMenu(Some(&menu));
    INSTALLED.set(Some(StatusMenu {
        _item: item,
        _menu: menu,
        _target: target,
    }));
}
