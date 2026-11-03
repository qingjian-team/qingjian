//! [`Strips`]：所有显示器四条边上的窗口。

use std::mem::size_of;

use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
};
use windows::core::BOOL;

use super::strip::{Side, Strip};

pub(super) struct Strips(Vec<Strip>);

impl Strips {
    pub(super) fn create() -> Self {
        let mut strips = Vec::new();
        for monitor in monitors() {
            let width = monitor.right - monitor.left;
            let height = monitor.bottom - monitor.top;
            let thickness = (width.min(height) / 40).clamp(20, 64);
            let sides = [
                (Side::Top, monitor.left, monitor.top, width, thickness),
                (
                    Side::Bottom,
                    monitor.left,
                    monitor.bottom - thickness,
                    width,
                    thickness,
                ),
                (Side::Left, monitor.left, monitor.top, thickness, height),
                (
                    Side::Right,
                    monitor.right - thickness,
                    monitor.top,
                    thickness,
                    height,
                ),
            ];
            for (side, x, y, w, h) in sides {
                if let Some(strip) = Strip::create(side, (x, y), (w, h), (width, height)) {
                    strips.push(strip);
                }
            }
        }
        Self(strips)
    }

    pub(super) fn draw(
        &self,
        color: [f32; 3],
        time: f32,
        strength: f32,
        mark: Option<(bool, f32)>,
    ) {
        for strip in &self.0 {
            strip.draw(color, time, strength, mark);
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
