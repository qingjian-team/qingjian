//! 中 / 英与大小写切换时的右上角提示（像搜狗那样）：一个不抢焦点的小浮窗，显示「中」「英」「大写」「小写」，一秒后自动收。
//!
//! 和候选窗同一层级、同样出现在所有 Space 上；不参与激活（`orderFrontRegardless`），不碰鼠标。

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSAttributedStringNSStringDrawing, NSBackingStoreType, NSBezierPath, NSColor, NSEvent, NSFont,
    NSFontAttributeName, NSForegroundColorAttributeName, NSPanel, NSScreen, NSView,
    NSWindowCollectionBehavior, NSWindowLevel, NSWindowStyleMask,
};
use objc2_foundation::{
    NSAttributedString, NSDictionary, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize,
    NSString, NSTimer,
};

/// 浮窗高度与最小宽度（点）。文字更长时按文字加两侧内边距撑宽。
const BOX_HEIGHT: f64 = 56.0;

const MIN_WIDTH: f64 = 56.0;

/// 文字两侧的内边距。
const PAD: f64 = 16.0;

/// 与屏幕上边缘、右边缘的间距。
const MARGIN: f64 = 14.0;

/// 显示多久（秒）。
const SHOW_SECONDS: f64 = 0.9;

/// 和候选窗同一层级（见 `candidates/window.rs` 的说明）。
const LEVEL: NSWindowLevel = 101;

/// 字号。
const FONT_SIZE: f64 = 24.0;

/// 角半径。
const CORNER: f64 = 12.0;

/// 右上角的中 / 英与大小写提示窗。
pub struct ModeOsd {
    panel: Retained<NSPanel>,

    view: Retained<ModeOsdView>,

    /// 收起用的定时器；显示期间有值。
    timer: Option<Retained<NSTimer>>,

    mtm: MainThreadMarker,
}

impl ModeOsd {
    pub fn new(mtm: MainThreadMarker) -> Self {
        let view = ModeOsdView::new(mtm);
        let panel = build_panel(mtm, &view);
        Self {
            panel,
            view,
            timer: None,
            mtm,
        }
    }

    /// 显示一段文字（「中」「英」「大写」「小写」）；已经在显示就换字并重新计时。
    pub fn show(&mut self, text: &str) {
        self.view.set_text(text);
        self.place(text);
        self.panel.orderFrontRegardless();
        self.view.setNeedsDisplay(true);
        if let Some(timer) = self.timer.take() {
            timer.invalidate();
        }
        let target = OsdTicker::new(self.mtm);
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                SHOW_SECONDS,
                &target,
                sel!(tick:),
                None,
                false,
            )
        };
        self.timer = Some(timer);
    }

    /// 收起来（没在显示时是空操作）。
    pub fn hide(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.invalidate();
        }
        self.panel.orderOut(None);
    }

    /// 摆到含鼠标那块屏幕的右上角，按文字宽度定窗宽。
    fn place(&self, text: &str) {
        let width = attributed(text, &text_font(), &NSColor::whiteColor())
            .size()
            .width
            .max(MIN_WIDTH - PAD * 2.0)
            + PAD * 2.0;
        let mouse = NSEvent::mouseLocation();
        let screens = NSScreen::screens(self.mtm);
        let screen = screens
            .iter()
            .find(|screen| {
                let frame = screen.frame();
                mouse.x >= frame.origin.x
                    && mouse.x < frame.origin.x + frame.size.width
                    && mouse.y >= frame.origin.y
                    && mouse.y < frame.origin.y + frame.size.height
            })
            .or_else(|| NSScreen::mainScreen(self.mtm));
        let Some(screen) = screen else {
            return;
        };
        let visible = screen.visibleFrame();
        let origin = NSPoint::new(
            visible.origin.x + visible.size.width - width - MARGIN,
            visible.origin.y + visible.size.height - BOX_HEIGHT - MARGIN,
        );
        self.panel
            .setFrame_display(NSRect::new(origin, NSSize::new(width, BOX_HEIGHT)), true);
    }
}

/// 提示文字用的字体。
fn text_font() -> Retained<NSFont> {
    NSFont::boldSystemFontOfSize(FONT_SIZE)
}

/// 字体 + 前景色的属性串（同 `candidates/view` 里的写法）。
fn attributed(text: &str, font: &NSFont, color: &NSColor) -> Retained<NSAttributedString> {
    // SAFETY: 只读 AppKit 导出的属性名常量
    let (keys, objects): (Vec<&NSString>, Vec<&AnyObject>) = unsafe {
        (
            vec![NSFontAttributeName, NSForegroundColorAttributeName],
            vec![font, color],
        )
    };
    let attributes = NSDictionary::from_slices(&keys, &objects);
    unsafe { NSAttributedString::new_with_attributes(&NSString::from_str(text), &attributes) }
}

fn build_panel(mtm: MainThreadMarker, view: &ModeOsdView) -> Retained<NSPanel> {
    let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
        mtm.alloc::<NSPanel>(),
        NSRect::new(NSPoint::ZERO, NSSize::new(MIN_WIDTH, BOX_HEIGHT)),
        NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
        NSBackingStoreType::Buffered,
        false,
    );
    panel.setOpaque(false);
    panel.setBackgroundColor(Some(&NSColor::clearColor()));
    // 圆角方块自己画，不要系统阴影
    panel.setHasShadow(false);
    panel.setBecomesKeyOnlyIfNeeded(true);
    panel.setIgnoresMouseEvents(true);
    // 输入法进程从来不是前台应用，别让它失活时自动隐藏
    panel.setHidesOnDeactivate(false);
    panel.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::Stationary,
    );
    panel.setLevel(LEVEL);
    panel.setContentView(Some(view));
    panel
}

struct Ivars {
    text: RefCell<String>,
}

define_class!(
    // SAFETY: NSView 允许子类化；没有实现 Drop。
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    struct ModeOsdView;

    impl ModeOsdView {
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            let bounds = self.bounds();
            NSColor::colorWithSRGBRed_green_blue_alpha(0.12, 0.12, 0.13, 0.78).set();
            NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(bounds, CORNER, CORNER).fill();
            let text = self.ivars().text.borrow();
            let string = attributed(&text, &text_font(), &NSColor::whiteColor());
            let size = string.size();
            string.drawAtPoint(NSPoint::new(
                (bounds.size.width - size.width) / 2.0,
                (bounds.size.height - size.height) / 2.0,
            ));
        }
    }
);

impl ModeOsdView {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(Ivars {
            text: RefCell::new(String::new()),
        });
        unsafe { msg_send![super(this), init] }
    }

    fn set_text(&self, text: &str) {
        *self.ivars().text.borrow_mut() = text.to_owned();
    }
}

define_class!(
    // SAFETY: NSObject 没有子类化要求；没有实现 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    struct OsdTicker;

    impl OsdTicker {
        #[unsafe(method(tick:))]
        fn tick(&self, _timer: Option<&AnyObject>) {
            // 定时器回调是 ObjC 运行时直接调的，panic 不能穿出去
            if crate::imk::catch_panic("中英提示收起", || crate::host::with(|h| h.osd.hide()))
                .is_none()
            {
                crate::imk::recover_from_panic(None);
            }
        }
    }

    unsafe impl NSObjectProtocol for OsdTicker {}
);

impl OsdTicker {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
