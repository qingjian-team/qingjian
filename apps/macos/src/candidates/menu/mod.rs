//! 候选右键菜单的窗口：候选窗之上一块不抢焦点的面板，菜单由 `qingjian-render` 画成位图贴上去（与 Windows 同一份画法）。
//! 左上角贴着鼠标，下面 / 右边放不下就往上 / 往左翻；点中一项、候选窗重画或收起时关掉。

mod view;

use std::rc::Rc;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSBackingStoreType, NSColor, NSPanel, NSWindowLevel, NSWindowStyleMask};
use objc2_foundation::{NSPoint, NSRect, NSSize};

use super::window::{POPUP_MENU_LEVEL, collection_behavior};
use view::MenuView;

/// 比候选窗高一层，盖得住它。
const MENU_LEVEL: NSWindowLevel = POPUP_MENU_LEVEL + 1;

/// 点中一项时调，参数是那一项的编号。
pub type MenuPick = Rc<dyn Fn(u32)>;

pub struct MenuPanel {
    panel: Retained<NSPanel>,

    view: Retained<MenuView>,
}

impl MenuPanel {
    pub fn new(mtm: MainThreadMarker) -> Self {
        let view = MenuView::new(mtm);
        let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
            mtm.alloc::<NSPanel>(),
            NSRect::new(NSPoint::ZERO, NSSize::new(120.0, 80.0)),
            NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
            NSBackingStoreType::Buffered,
            false,
        );
        panel.setOpaque(false);
        panel.setBackgroundColor(Some(&NSColor::clearColor()));
        panel.setHasShadow(false);
        panel.setBecomesKeyOnlyIfNeeded(true);
        panel.setHidesOnDeactivate(false);
        panel.setAcceptsMouseMovedEvents(true);
        panel.setCollectionBehavior(collection_behavior());
        panel.setLevel(MENU_LEVEL);
        panel.setContentView(Some(&view));
        Self { panel, view }
    }

    /// 在屏幕点 `at`（左下为原点）弹出菜单；`screen` 是那块屏幕的可见区域。画不出来就不弹。
    pub fn open(
        &self,
        items: Vec<(u32, &'static str)>,
        at: NSPoint,
        screen: NSRect,
        dark: bool,
        pick: MenuPick,
    ) {
        let scale = self.panel.backingScaleFactor() as f32;
        let Some(bounds) = self.view.open(items, dark, scale, pick) else {
            return;
        };
        let content = bounds.content;
        let mut x = at.x;
        if x + content.size.width > screen.origin.x + screen.size.width {
            x -= content.size.width;
        }
        let mut top = at.y;
        if top - content.size.height < screen.origin.y {
            top += content.size.height;
        }
        let origin = NSPoint::new(
            x - content.origin.x,
            top - content.size.height - content.origin.y,
        );
        self.panel
            .setFrame_display(NSRect::new(origin, bounds.size), true);
        self.view.setFrame(NSRect::new(NSPoint::ZERO, bounds.size));
        self.panel.orderFrontRegardless();
    }

    pub fn close(&self) {
        if self.panel.isVisible() {
            self.panel.orderOut(None);
            self.view.close();
        }
    }

    pub fn is_open(&self) -> bool {
        self.panel.isVisible()
    }
}
