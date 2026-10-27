//! 候选窗口：非激活的浮动 NSPanel，跟随光标，内容由 [`CandidateView`] 绘制。

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSBackingStoreType, NSColor, NSEvent, NSPanel, NSScreen, NSView, NSWindowCollectionBehavior,
    NSWindowLevel, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use qingjian_platform::{Appearance, CandidateRenderer, LayoutMode};

use super::backdrop::Backdrop;
use super::frame::Frame;
use super::key_hint::KeyHintPanel;
use super::theme::Theme;
use super::view::{CandidateView, ClickHandler, PanelMouseHandler};

/// `kCGPopUpMenuWindowLevel`：浮在普通窗口和浮动面板之上，与系统输入法候选框同级。
pub(super) const POPUP_MENU_LEVEL: NSWindowLevel = 101;

/// 候选窗口与光标行之间的间隙。
const CARET_GAP: f64 = 4.0;

/// 应用给不出光标位置时，拿鼠标位置当光标，按这个高度算一行。
const FALLBACK_LINE_HEIGHT: f64 = 16.0;

pub struct CandidateWindow {
    /// 面板本体。不在当前 Space 时会整个换新（见 [`Self::order_front_on_active_space`]）。
    panel: Retained<NSPanel>,

    /// 候选视图。
    view: Retained<CandidateView>,

    /// 面板的内容视图：候选视图装在里面，主题要系统材质时垫在它底下；换面板时整个搬过去。
    backdrop: Backdrop,

    /// 当前外观（跟随系统时为 `None`）；换面板时要重设。
    appearance: Option<Retained<NSAppearance>>,

    /// 双拼键位提示，跟着候选窗显示与收起。
    key_hint: KeyHintPanel,

    /// 这一段（候选窗从显示到收起）用过的光标行：应用中途给不出光标时沿用它，不追着鼠标走（#167）。
    last_anchor: Option<NSRect>,

    /// 用来取屏幕尺寸。
    mtm: MainThreadMarker,
}

impl CandidateWindow {
    pub fn new(mtm: MainThreadMarker) -> Self {
        let view = CandidateView::new(mtm, Theme::system_default());
        let backdrop = Backdrop::new(mtm, &view);
        let panel = build_panel(mtm, backdrop.container());
        Self {
            panel,
            view,
            backdrop,
            appearance: None,
            key_hint: KeyHintPanel::new(mtm),
            last_anchor: None,
            mtm,
        }
    }

    /// 显示一帧。`anchor` 是光标行的屏幕矩形，窗口贴在它下方，放不下就放上方。
    pub fn show(&mut self, frame: Frame, anchor: NSRect) {
        self.view.close_menu();
        if frame.is_empty() {
            self.hide();
            return;
        }
        let anchor = self.steady_anchor(anchor);
        let panel = frame.is_panel();
        let bounds = self.view.set_frame(&frame);
        let content = self.place(bounds.content.size, anchor);
        let keyboard = frame.keyboard;
        let origin = NSPoint::new(
            content.x - bounds.content.origin.x,
            content.y - bounds.content.origin.y,
        );
        let material = self.view.material();
        self.backdrop.update(material, bounds.content);
        self.panel
            .setFrame_display(NSRect::new(origin, bounds.size), true);
        self.order_front_on_active_space();
        self.apply_chrome(panel, material.is_some());
        match keyboard {
            Some(keyboard) => {
                let (caret, screen) = self.caret(anchor);
                let candidate = NSRect::new(content, bounds.content.size);
                self.key_hint.show(&keyboard, candidate, caret, screen);
            }
            None => self.key_hint.hide(),
        }
        if !self.panel.isVisible() {
            tracing::warn!(?anchor, ?origin, "候选窗口 orderFront 之后仍不可见");
        } else {
            tracing::debug!(
                ?anchor,
                ?origin,
                on_active_space = self.panel.isOnActiveSpace(),
                "候选窗口已显示"
            );
        }
    }

    /// 阴影与鼠标：系统绘制要系统阴影，青简渲染器的阴影画在位图里，垫了系统材质时又交回系统按窗口形状算；
    /// 两种候选画法与符号面板都收鼠标，面板本身仍是 `NonactivatingPanel`，不会抢走应用的键盘焦点。
    fn apply_chrome(&self, panel: bool, material: bool) {
        let bitmap = self.view.uses_bitmap() && !panel;
        self.panel.setHasShadow(!bitmap || material);
        if material {
            // 窗口形状每帧都变，系统阴影不会自己重算
            self.panel.invalidateShadow();
        }
        self.panel.setIgnoresMouseEvents(false);
    }

    pub fn hide(&mut self) {
        self.last_anchor = None;
        self.view.close_menu();
        self.panel.orderOut(None);
        self.key_hint.hide();
        self.view.stop_animation();
    }

    /// 排到最前，并确认真在当前 Space 上；不在就换一块新面板。
    ///
    /// collection behavior 是「所有 Space + 全屏辅助」，可 macOS 26 上 WindowServer 只把面板绑到它创建时已有的 Space：
    /// 之后新开的全屏 Space（Zed 全屏就是一个）里没有它，候选框留在桌面那个 Space。用 CGS 查过，面板的 Space 列表只有桌面，
    /// 而同样设置新建的面板会绑到全部 Space；重设 collection behavior、收起再排前都不会让它重算，只有新建有用。
    /// `isOnActiveSpace` 能反映这个状态，所以每次显示后查一下，不在就把内容视图搬到新面板上。
    fn order_front_on_active_space(&mut self) {
        self.panel.orderFrontRegardless();
        if self.panel.isOnActiveSpace() {
            return;
        }
        let frame = self.panel.frame();
        self.panel.orderOut(None);
        let panel = build_panel(self.mtm, self.backdrop.container());
        panel.setAppearance(self.appearance.as_deref());
        panel.setHasShadow(!self.view.uses_bitmap());
        panel.setIgnoresMouseEvents(false);
        panel.setFrame_display(frame, true);
        panel.orderFrontRegardless();
        self.panel = panel;
        tracing::warn!(
            recovered = self.panel.isOnActiveSpace(),
            "候选窗口不在当前 Space，已换新面板"
        );
    }

    /// 外观：跟随系统时不指定，否则强制浅色 / 深色。
    pub fn set_appearance(&mut self, mode: Appearance) {
        // SAFETY: 只读 AppKit 导出的常量名
        let name = unsafe {
            match mode {
                Appearance::System => None,
                Appearance::Light => Some(NSAppearanceNameAqua),
                Appearance::Dark => Some(NSAppearanceNameDarkAqua),
            }
        };
        let appearance = name.and_then(NSAppearance::appearanceNamed);
        self.panel.setAppearance(appearance.as_deref());
        self.key_hint.set_appearance(appearance.clone());
        self.appearance = appearance;
    }

    /// 键位提示窗口的不透明度（0–1）。
    pub fn set_key_hint_opacity(&mut self, opacity: f32) {
        self.key_hint.set_opacity(opacity);
    }

    /// 键位提示键盘的大小（1 为原大）。
    pub fn set_key_hint_size(&mut self, zoom: f32) {
        self.key_hint.set_zoom(zoom);
    }

    /// 竖排 / 横排。下一帧生效。
    pub fn set_layout(&self, layout: LayoutMode) {
        self.view.set_layout(layout);
    }

    /// 青简渲染器 / 系统绘制。下一帧生效。青简渲染器的阴影由主题画进位图，系统绘制用系统阴影；
    /// 两种画法都有点击区域并接收鼠标。
    pub fn set_renderer(&self, renderer: CandidateRenderer) {
        self.view.set_renderer(renderer);
        self.panel.setHasShadow(!self.view.uses_bitmap());
        self.panel.setIgnoresMouseEvents(false);
    }

    /// 点中候选或译词时调谁。
    pub fn set_click_handler(&self, handler: ClickHandler) {
        self.view.set_click_handler(handler);
    }

    /// 右键菜单开着（敲 Esc 先关它，不清输入）。
    pub fn menu_open(&self) -> bool {
        self.view.menu_open()
    }

    pub fn close_menu(&self) {
        self.view.close_menu();
    }

    /// 右键候选时问谁给菜单项、点中的项交给谁。
    pub fn set_menu_handlers(&self, handlers: super::view::MenuHandlers) {
        self.view.set_menu_handlers(handlers);
    }

    /// 符号面板里点中格子或标签时调谁。
    pub fn set_panel_mouse_handler(&self, handler: PanelMouseHandler) {
        self.view.set_panel_mouse_handler(handler);
    }

    /// 渲染器主题（按 `[general] theme` 从主题库取出的），只对青简渲染器生效。
    pub fn set_render_theme(&mut self, theme: qingjian_render::Theme) {
        self.key_hint.set_theme(theme.clone());
        self.view.set_render_theme(theme);
    }

    /// 候选窗字体（字族名，空为系统字体），只对青简渲染器生效。
    pub fn set_font(&self, font: &str) {
        self.view.set_font(font);
    }

    pub fn max_rows(&self) -> usize {
        self.view.theme().max_rows
    }

    /// 内容区左下角坐标：贴在光标行下方；下方放不下放上方；不出光标所在的那块屏幕。
    /// 光标矩形是零或落在所有屏幕之外（应用不支持、或给的是胡话）时以鼠标位置为准，至少落在用户看着的屏幕上。
    fn place(&self, size: NSSize, anchor: NSRect) -> NSPoint {
        let (anchor, screen) = self.caret(anchor);
        let min_x = screen.origin.x;
        let max_x = (screen.origin.x + screen.size.width - size.width).max(min_x);
        let x = anchor.origin.x.clamp(min_x, max_x);
        let below = anchor.origin.y - CARET_GAP - size.height;
        let above = anchor.origin.y + anchor.size.height + CARET_GAP;
        let top = screen.origin.y + screen.size.height;
        let y = if below >= screen.origin.y {
            below
        } else if above + size.height <= top {
            above
        } else {
            // 上下都放不下（屏幕很矮或窗口很高）：贴屏幕底边，宁可盖住光标也别出屏
            screen.origin.y
        };
        // 无论怎么算，最后都要落在这块屏幕里：出屏等于不显示
        let max_y = (top - size.height).max(screen.origin.y);
        NSPoint::new(x, y.clamp(screen.origin.y, max_y))
    }

    /// 应用给得出光标就用它并记下；给不出时沿用这一段记下的，一段里第一次就给不出才用鼠标位置，同样记下。
    fn steady_anchor(&mut self, anchor: NSRect) -> NSRect {
        let known = !(anchor.size.height == 0.0 && anchor.origin == NSPoint::ZERO)
            && screen_containing(self.mtm, anchor.origin).is_some();
        let anchor = match (known, self.last_anchor) {
            (true, _) => anchor,
            (false, Some(last)) => last,
            (false, None) => NSRect::new(
                NSEvent::mouseLocation(),
                NSSize::new(0.0, FALLBACK_LINE_HEIGHT),
            ),
        };
        self.last_anchor = Some(anchor);
        anchor
    }

    /// 光标行与它所在屏幕的可见区域。光标矩形是零或落在所有屏幕之外（应用不支持、或给的是胡话）时以鼠标位置为准。
    fn caret(&self, anchor: NSRect) -> (NSRect, NSRect) {
        match screen_containing(self.mtm, anchor.origin) {
            Some(screen) if !(anchor.size.height == 0.0 && anchor.origin == NSPoint::ZERO) => {
                (anchor, screen)
            }
            _ => {
                let mouse = NSEvent::mouseLocation();
                let anchor = NSRect::new(mouse, NSSize::new(0.0, FALLBACK_LINE_HEIGHT));
                (
                    anchor,
                    screen_containing(self.mtm, mouse)
                        .unwrap_or_else(|| main_screen_or_anywhere(self.mtm)),
                )
            }
        }
    }
}

/// 建一块面板并把内容视图装进去：无边框、不抢焦点、透明背景带系统阴影（用青简渲染器时再关）、
/// 接收鼠标但不激活应用，候选的两种绘制路径都支持点击与右键菜单。
fn build_panel(mtm: MainThreadMarker, view: &NSView) -> Retained<NSPanel> {
    let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
        mtm.alloc::<NSPanel>(),
        NSRect::new(NSPoint::ZERO, NSSize::new(200.0, 100.0)),
        NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
        NSBackingStoreType::Buffered,
        false,
    );
    panel.setOpaque(false);
    panel.setBackgroundColor(Some(&NSColor::clearColor()));
    panel.setHasShadow(true);
    panel.setBecomesKeyOnlyIfNeeded(true);
    panel.setIgnoresMouseEvents(false);
    // NSPanel 缺省在应用失活时自动隐藏；输入法进程从来不是前台应用，不能靠这个
    panel.setHidesOnDeactivate(false);
    panel.setCollectionBehavior(collection_behavior());
    // 不要 setFloatingPanel(true)：它会把层级改回 NSFloatingWindowLevel（3），全屏应用的 Space 里就看不见了；
    // 层级最后设，别被前面任何一项覆盖
    panel.setLevel(POPUP_MENU_LEVEL);
    panel.setContentView(Some(view));
    panel
}

/// 面板的 Space 归属：出现在所有 Space 上、能与全屏应用同处一个 Space、Mission Control 里不动。
pub(super) fn collection_behavior() -> NSWindowCollectionBehavior {
    NSWindowCollectionBehavior::CanJoinAllSpaces
        | NSWindowCollectionBehavior::FullScreenAuxiliary
        | NSWindowCollectionBehavior::Stationary
}

/// 包含 `point` 的那块屏幕的可见区域；哪块都不包含返回 `None`。
fn screen_containing(mtm: MainThreadMarker, point: NSPoint) -> Option<NSRect> {
    let screens = NSScreen::screens(mtm);
    let hit = screens.iter().find(|screen| {
        let frame = screen.frame();
        point.x >= frame.origin.x
            && point.x < frame.origin.x + frame.size.width
            && point.y >= frame.origin.y
            && point.y < frame.origin.y + frame.size.height
    });
    hit.map(|screen| screen.visibleFrame())
}

/// 主屏的可见区域；连主屏都没有时不限制。
fn main_screen_or_anywhere(mtm: MainThreadMarker) -> NSRect {
    NSScreen::mainScreen(mtm)
        .map(|s| s.visibleFrame())
        .unwrap_or_else(|| {
            NSRect::new(
                NSPoint::new(f64::MIN / 2.0, f64::MIN / 2.0),
                NSSize::new(f64::MAX, f64::MAX),
            )
        })
}
