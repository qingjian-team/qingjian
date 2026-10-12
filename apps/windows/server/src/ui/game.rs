//! 游戏模式的独立候选锚点：只读前台窗口与宿主路径，不访问游戏内存或安装钩子。

use windows::Win32::Foundation::{CloseHandle, RECT};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId, IsIconic,
};
use windows::core::PWSTR;

/// 命令到达时再次核对宿主，避免异步翻译在切到另一应用后仍显示游戏候选。
pub(super) fn foreground_bounds(app: &str) -> Option<RECT> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() || unsafe { IsIconic(hwnd) }.as_bool() {
        return None;
    }
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
    let mut name = [0_u16; 32768];
    let mut len = name.len() as u32;
    let queried = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(name.as_mut_ptr()),
            &mut len,
        )
    };
    let _ = unsafe { CloseHandle(process) };
    queried.ok()?;
    let path = String::from_utf16_lossy(&name[..len as usize]);
    let basename = path.rsplit(['\\', '/']).next()?;
    if !basename.eq_ignore_ascii_case(app) {
        return None;
    }
    let mut bounds = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut bounds) }.ok()?;
    let width = bounds.right - bounds.left;
    let height = bounds.bottom - bounds.top;
    if width <= 0 || height <= 0 {
        return None;
    }
    Some(bounds)
}

pub(super) fn foreground_anchor(app: &str) -> Option<RECT> {
    let bounds = foreground_bounds(app)?;
    let x = bounds.left + (bounds.right - bounds.left) / 12;
    let y = bounds.top + (bounds.bottom - bounds.top) * 3 / 5;
    Some(RECT {
        left: x,
        top: y,
        right: x + 1,
        bottom: y + 1,
    })
}
