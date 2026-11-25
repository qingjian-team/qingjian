//! 显示器工作区与 DPI 查询：进程内候选窗按光标所在显示器摆位、按它的缩放倍数出图。
//!
//! DLL 跑在宿主应用的 DPI 上下文里，屏幕坐标也按宿主那一套来；这里只问「光标在哪块屏、它多密」，
//! 与 Server 侧 `ui/monitor.rs` 同一个做法。

use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITOR_FROM_FLAGS, MONITORINFO, MonitorFromPoint,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

/// `point` 所在（最近）显示器的工作区。
pub(super) fn work_area_near(point: POINT) -> RECT {
    work_area(point, MONITOR_DEFAULTTONEAREST)
}

/// 拿不到时给个大范围，至少别把窗口夹没。
fn work_area(point: POINT, flags: MONITOR_FROM_FLAGS) -> RECT {
    let mut info = MONITORINFO {
        cbSize: core::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let found = unsafe {
        let monitor = MonitorFromPoint(point, flags);
        GetMonitorInfoW(monitor, &mut info).as_bool()
    };
    if found {
        info.rcWork
    } else {
        RECT {
            left: 0,
            top: 0,
            right: i32::MAX,
            bottom: i32::MAX,
        }
    }
}

/// `point` 所在（最近）显示器的有效 DPI；取不到为 `None`。
pub(super) fn dpi_near(point: POINT) -> Option<u32> {
    let (mut x, mut y) = (0, 0);
    unsafe {
        let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
        GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut x, &mut y).ok()?;
    }
    Some(x.max(96))
}
