//! 光标所在显示器的可用区域，换成面板窗口用的物理像素。

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSApplication, NSEvent, NSScreen, NSWindow};
use objc2_core_foundation::{
    CFArray, CFDictionary, CFNumber, CFRetained, CFString, CFType, CGRect,
};
use objc2_core_graphics::{
    CGRectMakeWithDictionaryRepresentation, CGWindowListCopyWindowInfo, CGWindowListOption,
    kCGNullWindowID, kCGWindowBounds, kCGWindowLayer, kCGWindowOwnerPID,
};
use objc2_foundation::{NSPoint, NSRect};
use qingjian_clipboard::WorkArea;

/// 窗口宿主开的面板窗口（winit 的 `WinitWindow`）。本进程还有菜单栏图标与霓虹灯的窗口，不能当成它。
pub fn panel_window(mtm: MainThreadMarker) -> Option<Retained<NSWindow>> {
    NSApplication::sharedApplication(mtm)
        .windows()
        .into_iter()
        .find(|window| window.class().name() == c"WinitWindow")
}

/// 光标所在显示器去掉菜单栏与程序坞的区域，以及点 → 像素的倍数；取不到时按 1440×900、2 倍兜底。
/// 那块屏幕前台是全屏应用时程序坞与菜单栏都藏着，`visibleFrame` 却照样扣掉它们，这时改用整块屏幕。
///
/// 窗口宿主按面板窗口**现在**所在屏幕的倍数把像素换回点，所以倍数取面板窗口的，不取目标屏幕的：
/// 两块屏幕倍数不同时，换算出来的点仍然对得上，挪过去之后窗口宿主再按新屏幕重画。
pub fn cursor_screen(mtm: MainThreadMarker) -> (WorkArea, f32) {
    let screens = NSScreen::screens(mtm);
    let mouse = NSEvent::mouseLocation();
    let contains = |screen: &NSScreen, point: NSPoint| {
        let frame = screen.frame();
        point.x >= frame.origin.x
            && point.x <= frame.origin.x + frame.size.width
            && point.y >= frame.origin.y
            && point.y <= frame.origin.y + frame.size.height
    };
    let target = screens
        .iter()
        .find(|screen| contains(screen, mouse))
        .or_else(|| screens.firstObject());
    let (Some(target), Some(primary)) = (target, screens.firstObject()) else {
        return (
            WorkArea {
                left: 0,
                top: 0,
                right: 2880,
                bottom: 1800,
            },
            2.0,
        );
    };
    let scale = panel_window(mtm).map_or_else(
        || target.backingScaleFactor(),
        |window| window.backingScaleFactor(),
    ) as f32;
    // AppKit 的坐标原点在主屏左下、y 朝上；窗口宿主的在主屏左上、y 朝下
    let primary_height = primary.frame().size.height;
    let visible = if fullscreen_on(target.frame(), primary_height) {
        target.frame()
    } else {
        target.visibleFrame()
    };
    let to_pixels = |value: f64| (value * f64::from(scale)).round() as i32;
    let top = primary_height - (visible.origin.y + visible.size.height);
    let area = WorkArea {
        left: to_pixels(visible.origin.x),
        top: to_pixels(top),
        right: to_pixels(visible.origin.x + visible.size.width),
        bottom: to_pixels(top + visible.size.height),
    };
    (area, scale)
}

/// 别的应用有一个铺满这块屏幕的普通层窗口，就是全屏空间。`screen` 是 AppKit 坐标，窗口列表的是主屏左上为原点、y 朝下。
/// 只读窗口的层级与位置，不要屏幕录制权限。
fn fullscreen_on(screen: NSRect, primary_height: f64) -> bool {
    let options =
        CGWindowListOption::OptionOnScreenOnly | CGWindowListOption::ExcludeDesktopElements;
    let Some(windows) = CGWindowListCopyWindowInfo(options, kCGNullWindowID) else {
        return false;
    };
    // SAFETY: 窗口列表的每一项都是以 CFString 为键的字典
    let windows: CFRetained<CFArray<CFDictionary<CFString, CFType>>> =
        unsafe { CFRetained::cast_unchecked(windows) };
    let top = primary_height - (screen.origin.y + screen.size.height);
    let own = i64::from(std::process::id());
    let number = |window: &CFDictionary<CFString, CFType>, key: &CFString| {
        window
            .get(key)
            .and_then(|value| value.downcast::<CFNumber>().ok())
            .and_then(|value| value.as_i64())
    };
    windows.iter().any(|window| {
        // SAFETY: 这几个键是 CoreGraphics 导出的常量
        let (layer, owner, bounds) =
            unsafe { (kCGWindowLayer, kCGWindowOwnerPID, kCGWindowBounds) };
        if number(&window, layer) != Some(0) || number(&window, owner) == Some(own) {
            return false;
        }
        let Some(bounds) = window
            .get(bounds)
            .and_then(|value| value.downcast::<CFDictionary>().ok())
        else {
            return false;
        };
        let mut rect = CGRect::default();
        // SAFETY: rect 指向一块有效的 CGRect
        if !unsafe { CGRectMakeWithDictionaryRepresentation(Some(&bounds), &mut rect) } {
            return false;
        }
        let close = |a: f64, b: f64| (a - b).abs() < 1.0;
        close(rect.origin.x, screen.origin.x)
            && close(rect.origin.y, top)
            && close(rect.size.width, screen.size.width)
            && close(rect.size.height, screen.size.height)
    })
}
