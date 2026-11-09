//! [`Strips`]：所有显示器四条边上的窗口，以及什么时候不亮、不流动。

use std::mem::size_of;

use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
};
use windows::Win32::UI::Shell::{
    QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN, SHQueryUserNotificationState,
};
use windows::Win32::UI::WindowsAndMessaging::{
    SPI_GETCLIENTAREAANIMATION, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
};
use windows::core::BOOL;

use super::neon_strip::Strip;
use crate::system::neon::Edge;

/// 灯条厚度的上下限（像素）。
const THICKNESS: (i32, i32) = (20, 64);

pub(crate) struct Strips(Vec<Strip>);

impl Strips {
    pub(crate) fn create() -> Self {
        let mut strips = Vec::new();
        for monitor in monitors() {
            let size = (monitor.right - monitor.left, monitor.bottom - monitor.top);
            for edge in Edge::around(size, THICKNESS) {
                let (x, y) = edge.offset();
                if let Some(strip) = Strip::create(edge, (monitor.left + x, monitor.top + y)) {
                    strips.push(strip);
                }
            }
        }
        Self(strips)
    }

    pub(crate) fn draw(&self, color: [f32; 3], time: f32, strength: f32) {
        for strip in &self.0 {
            strip.draw(color, time, strength);
        }
    }
}

fn monitors() -> Vec<RECT> {
    unsafe extern "system" fn collect(
        monitor: HMONITOR,
        _: HDC,
        _: *mut RECT,
        list: LPARAM,
    ) -> BOOL {
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..MONITORINFO::default()
        };
        // SAFETY: list 是下面传进来的 Vec，枚举期间一直活着。
        unsafe {
            if GetMonitorInfoW(monitor, &mut info).as_bool() {
                (*(list.0 as *mut Vec<RECT>)).push(info.rcMonitor);
            }
        }
        BOOL(1)
    }
    let mut list: Vec<RECT> = Vec::new();
    // SAFETY: 回调只在这次调用内用这个指针。
    let _ =
        unsafe { EnumDisplayMonitors(None, None, Some(collect), LPARAM((&raw mut list) as isize)) };
    list
}

/// 全屏游戏、演示时不打扰。
pub(crate) fn should_hide() -> bool {
    let Ok(state) = (unsafe { SHQueryUserNotificationState() }) else {
        return false;
    };
    state == QUNS_RUNNING_D3D_FULL_SCREEN || state == QUNS_PRESENTATION_MODE || state == QUNS_BUSY
}

/// 系统「显示动画」关了就不流动，只亮和变色。
pub(crate) fn animations_on() -> bool {
    let mut on = BOOL(1);
    let read = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some((&raw mut on).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    read.is_err() || on.as_bool()
}
