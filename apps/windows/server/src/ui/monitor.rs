//! 显示器工作区查询，候选窗口与状态条共用。

use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTOPRIMARY, MONITOR_FROM_FLAGS,
    MONITORINFO, MonitorFromPoint,
};

/// `point` 所在（最近）显示器的工作区。
pub(super) fn work_area_near(point: POINT) -> RECT {
    work_area(point, MONITOR_DEFAULTTONEAREST)
}

/// 主显示器的工作区。
pub(super) fn primary_work_area() -> RECT {
    work_area(POINT::default(), MONITOR_DEFAULTTOPRIMARY)
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
