//! 双拼键位提示窗口：候选窗旁一块不抢焦点、点击穿透的面板，键盘由 `qingjian-render` 画成位图贴上去。
//! 跟着候选窗显示与收起（[`super::CandidateWindow`] 管），透明度设在窗口上，拖滑条不用重画。

mod place;

use std::cell::Cell;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSBackingStoreType, NSColor, NSImageView, NSPanel, NSWindowStyleMask,
};
use objc2_foundation::{NSArray, NSPoint, NSRect, NSSize};
use qingjian_render::{FontLibrary, Keyboard, Renderer, Theme};

use super::bitmap::{bounds, family_files, to_image};
use super::window::{POPUP_MENU_LEVEL, collection_behavior};
use place::Side;

pub struct KeyHintPanel {
    /// 面板本体。不在当前 Space 时整个换新，同候选窗。
    panel: Retained<NSPanel>,

    image: Retained<NSImageView>,

    /// 第一次显示时才建（加载字体库要几十毫秒，不开键位提示的人不花这个钱）；建不成为 `None`，不再试。
    renderer: Option<Option<Renderer>>,

    /// 渲染器主题（浅色那一份）。
    theme: Theme,

    appearance: Option<Retained<NSAppearance>>,

    /// 窗口不透明度（0–1）。
    opacity: f64,

    /// 键盘大小（1 为原大）。
    zoom: f32,

    /// 这一段显示里用的那一侧，收起时忘掉；下次显示先试它，键盘不随光标左右跳。
    side: Cell<Option<Side>>,

    mtm: MainThreadMarker,
}

impl KeyHintPanel {
    pub fn new(mtm: MainThreadMarker) -> Self {
        let image = NSImageView::new(mtm);
        let panel = build_panel(mtm, &image);
        Self {
            panel,
            image,
            renderer: None,
            theme: Theme::light(),
            appearance: None,
            opacity: 1.0,
            zoom: 1.0,
            side: Cell::new(None),
            mtm,
        }
    }

    /// 画好键盘贴在候选窗旁边：`candidate` 是候选窗内容区，`caret` 是光标行，`screen` 是那块屏幕的可见区域。
    /// 哪儿都放不下就收起，这次不显示。
    pub fn show(&mut self, keyboard: &Keyboard, candidate: NSRect, caret: NSRect, screen: NSRect) {
        let dark = self.is_dark();
        let scale = self.panel.backingScaleFactor() as f32;
        let theme = self.theme.with_dark(dark);
        let zoom = self.zoom;
        let Some(renderer) = self.renderer() else {
            return;
        };
        let rendered = match renderer.render_keyboard(keyboard, &theme, scale, zoom) {
            Ok(rendered) => rendered,
            Err(error) => {
                tracing::warn!(%error, "键位提示渲染失败");
                self.hide();
                return;
            }
        };
        let bounds = bounds(&rendered);
        let placed = place::place(
            bounds.content.size,
            candidate,
            caret,
            screen,
            self.side.get(),
        );
        let Some((content, side)) = placed else {
            tracing::debug!(?candidate, ?caret, "键位提示放不下，这次不显示");
            self.hide();
            return;
        };
        self.side.set(Some(side));
        let origin = NSPoint::new(
            content.x - bounds.content.origin.x,
            content.y - bounds.content.origin.y,
        );
        self.image
            .setImage(to_image(&rendered.pixmap, bounds.size).as_deref());
        self.panel
            .setFrame_display(NSRect::new(origin, bounds.size), true);
        self.image.setFrame(NSRect::new(NSPoint::ZERO, bounds.size));
        self.panel.setAlphaValue(self.opacity);
        self.order_front_on_active_space();
    }

    pub fn hide(&self) {
        self.panel.orderOut(None);
        self.side.set(None);
    }

    pub fn set_theme(&mut self, theme: Theme) {
        if let Some(Some(renderer)) = &mut self.renderer {
            renderer.load_theme_fonts(&theme, family_files);
        }
        self.theme = theme;
    }

    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = f64::from(opacity);
        self.panel.setAlphaValue(self.opacity);
    }

    /// 下一次显示生效。
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom;
    }

    pub fn set_appearance(&mut self, appearance: Option<Retained<NSAppearance>>) {
        self.panel.setAppearance(appearance.as_deref());
        self.appearance = appearance;
    }

    fn renderer(&mut self) -> Option<&mut Renderer> {
        let theme = &self.theme;
        self.renderer
            .get_or_insert_with(|| match FontLibrary::system("zh-CN") {
                Ok(library) => {
                    let mut renderer = Renderer::new(library);
                    renderer.load_theme_fonts(theme, family_files);
                    Some(renderer)
                }
                Err(error) => {
                    tracing::warn!(%error, "键位提示的字体库加载失败，不显示键位提示");
                    None
                }
            })
            .as_mut()
    }

    fn is_dark(&self) -> bool {
        // SAFETY: 只读 AppKit 导出的常量名
        let names =
            unsafe { NSArray::from_slice(&[NSAppearanceNameAqua, NSAppearanceNameDarkAqua]) };
        let matched = self
            .panel
            .effectiveAppearance()
            .bestMatchFromAppearancesWithNames(&names);
        // SAFETY: 同上
        matched.is_some_and(|name| unsafe { name.isEqualToString(NSAppearanceNameDarkAqua) })
    }

    /// 同 [`super::CandidateWindow`]：macOS 26 上面板只绑在创建时已有的 Space，新开的全屏 Space 里要换新面板。
    fn order_front_on_active_space(&mut self) {
        self.panel.orderFrontRegardless();
        if self.panel.isOnActiveSpace() {
            return;
        }
        let frame = self.panel.frame();
        self.panel.orderOut(None);
        let panel = build_panel(self.mtm, &self.image);
        panel.setAppearance(self.appearance.as_deref());
        panel.setAlphaValue(self.opacity);
        panel.setFrame_display(frame, true);
        panel.orderFrontRegardless();
        self.panel = panel;
    }
}

/// 无边框、不抢焦点、透明背景、点击穿透；阴影由主题画进位图，系统阴影关掉。
fn build_panel(mtm: MainThreadMarker, image: &NSImageView) -> Retained<NSPanel> {
    let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
        mtm.alloc::<NSPanel>(),
        NSRect::new(NSPoint::ZERO, NSSize::new(200.0, 100.0)),
        NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
        NSBackingStoreType::Buffered,
        false,
    );
    panel.setOpaque(false);
    panel.setBackgroundColor(Some(&NSColor::clearColor()));
    panel.setHasShadow(false);
    panel.setBecomesKeyOnlyIfNeeded(true);
    panel.setIgnoresMouseEvents(true);
    panel.setHidesOnDeactivate(false);
    panel.setCollectionBehavior(collection_behavior());
    panel.setLevel(POPUP_MENU_LEVEL);
    panel.setContentView(Some(image));
    panel
}
