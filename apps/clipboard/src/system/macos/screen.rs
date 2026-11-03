//! 光标所在显示器的可用区域，换成面板窗口用的物理像素。

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSApplication, NSEvent, NSScreen, NSWindow};
use objc2_foundation::NSPoint;
use qingjian_clipboard::WorkArea;

/// 窗口宿主开的面板窗口（winit 的 `WinitWindow`）。本进程还有菜单栏图标与霓虹灯的窗口，不能当成它。
pub fn panel_window(mtm: MainThreadMarker) -> Option<Retained<NSWindow>> {
    NSApplication::sharedApplication(mtm)
        .windows()
        .into_iter()
        .find(|window| window.class().name() == c"WinitWindow")
}

/// 光标所在显示器去掉菜单栏与程序坞的区域，以及点 → 像素的倍数；取不到时按 1440×900、2 倍兜底。
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
    let visible = target.visibleFrame();
    let primary_height = primary.frame().size.height;
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
