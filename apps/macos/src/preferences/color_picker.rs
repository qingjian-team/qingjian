//! 原生选色控件与单项恢复默认按钮，配置沿用设置页的即时保存通路。

use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSButton, NSColor, NSColorSpace, NSColorWell,
    NSTextField,
};
use objc2_foundation::{NSObjectProtocol, NSRect, NSString};

use crate::candidates::colors::{format_hex, parse_hex};
use crate::preferences::controls::{button, caption, small_label};
use crate::preferences::layout::{CONTROL_X, LABEL_WIDTH, Layout, PAGE_PADDING, ROW_HEIGHT};
use crate::preferences::setting::Setting;
use crate::preferences::target::PreferencesTarget;
use qingjian_render::Color;

pub(super) struct ColorPicker {
    well: Retained<NSColorWell>,

    value: Retained<NSTextField>,

    reset: Retained<NSButton>,
}

impl ColorPicker {
    pub fn build(
        layout: &mut Layout,
        mtm: MainThreadMarker,
        title: &str,
        setting: Setting,
        target: &PreferencesTarget,
    ) -> Self {
        let label = caption(mtm, title);
        layout.place(&label, PAGE_PADDING, LABEL_WIDTH, ROW_HEIGHT);
        let well = NSColorWell::initWithFrame(mtm.alloc(), NSRect::ZERO);
        well.setTag(setting.tag());
        well.setContinuous(true);
        // 默认文字与高亮本来就带透明度，取色与保存必须保留这一层。
        if well.respondsToSelector(sel!(setSupportsAlpha:)) {
            well.setSupportsAlpha(true);
        }
        // SAFETY: 与 PreferencesTarget::changed: 的签名一致，target 由窗口持有。
        unsafe {
            well.setTarget(Some(target));
            well.setAction(Some(sel!(changed:)));
        }
        well.setToolTip(Some(&NSString::from_str(&format!("选择{title}"))));
        layout.place(&well, CONTROL_X, 56.0, ROW_HEIGHT);
        let value = small_label(mtm, "默认");
        layout.place(&value, CONTROL_X + 68.0, 100.0, ROW_HEIGHT);
        let reset = button(mtm, "恢复默认", setting, target);
        layout.place(&reset, CONTROL_X + 180.0, 100.0, ROW_HEIGHT);
        layout.next_row(ROW_HEIGHT);
        Self { well, value, reset }
    }

    pub fn sync(&self, configured: &str, color: &NSColor, appearance: &NSAppearance) {
        self.well.setAppearance(Some(appearance));
        let custom = parse_hex(configured);
        if custom.is_none() {
            self.well.deactivate();
        }
        // 拖动取色器时保留其连续值，避免 8 位量化结果反向推动正在操作的滑块。
        if !self.well.isActive() {
            self.well.setColor(color);
        }
        let label = custom.map_or_else(|| "默认".into(), format_hex);
        self.value.setStringValue(&NSString::from_str(&label));
        self.reset.setEnabled(!configured.trim().is_empty());
    }
}

pub(super) fn selected_hex(well: &NSColorWell) -> Option<String> {
    let rgb = well
        .color()
        .colorUsingColorSpace(&NSColorSpace::sRGBColorSpace())?;
    let byte = |value: f64| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    Some(format_hex(Color::rgba(
        byte(rgb.redComponent()),
        byte(rgb.greenComponent()),
        byte(rgb.blueComponent()),
        byte(rgb.alphaComponent()),
    )))
}
