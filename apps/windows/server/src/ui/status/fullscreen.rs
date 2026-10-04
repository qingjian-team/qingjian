//! 前台是全屏应用（游戏、看视频、演示）时收起状态条，退出全屏再放出来。
//! 按 F11 进全屏不换前台窗口、没有事件可等，状态条显示期间每秒看一次。

use core::cell::Cell;

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_STYLE, GetForegroundWindow, GetShellWindow, GetWindowLongW, GetWindowRect, IsZoomed,
    KillTimer, SW_HIDE, SW_SHOWNA, SetTimer, ShowWindow, WS_CAPTION,
};

/// 状态条窗口上的定时器编号。
pub(super) const TIMER_ID: usize = 1;

const INTERVAL_MS: u32 = 1000;

/// 显示状态条，前台正全屏就先收着；开始每秒检查。
pub(super) fn show(hwnd: HWND, hidden: &Cell<bool>) {
    unsafe { SetTimer(Some(hwnd), TIMER_ID, INTERVAL_MS, None) };
    hidden.set(false);
    on_timer(hwnd, hidden);
    if !hidden.get() {
        let _ = unsafe { ShowWindow(hwnd, SW_SHOWNA) };
    }
}

/// 收起状态条（青简不在前台 / 关掉了），不再检查。
pub(super) fn hide(hwnd: HWND, hidden: &Cell<bool>) {
    let _ = unsafe { KillTimer(Some(hwnd), TIMER_ID) };
    hidden.set(false);
    let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };
}

/// 每秒一次：进全屏收起，出全屏放回。
pub(super) fn on_timer(hwnd: HWND, hidden: &Cell<bool>) {
    let fullscreen = foreground_is_fullscreen();
    if fullscreen != hidden.get() {
        hidden.set(fullscreen);
        let command = if fullscreen { SW_HIDE } else { SW_SHOWNA };
        let _ = unsafe { ShowWindow(hwnd, command) };
    }
}

/// 前台窗口盖满了它所在的整个显示器（连任务栏一起）。带标题栏的最大化窗口不算：
/// 任务栏自动隐藏时它也盖满整屏，但用户要的是照常显示。
///
/// 候选窗口定位也依赖这个判断：全屏（游戏/视频/演示）时 TSF 上报的组句矩形坐标
/// 经常滞后或停留在主屏，候选窗会贴到错误的显示器上（#360），需要换用真实光标。
pub(super) fn foreground_is_fullscreen() -> bool {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() || hwnd == unsafe { GetShellWindow() } {
        return false;
    }
    let zoomed_with_caption = unsafe { IsZoomed(hwnd) }.as_bool()
        && unsafe { GetWindowLongW(hwnd, GWL_STYLE) } as u32 & WS_CAPTION.0 == WS_CAPTION.0;
    if zoomed_with_caption {
        return false;
    }
    let mut window = RECT::default();
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    unsafe {
        if GetWindowRect(hwnd, &mut window).is_err() {
            return false;
        }
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return false;
        }
    }
    let screen = info.rcMonitor;
    window.left <= screen.left
        && window.top <= screen.top
        && window.right >= screen.right
        && window.bottom >= screen.bottom
}
