//! Windows 上设置程序只开一个：第二次启动把已开的窗口带到前台后退出（旧设置程序同一套做法，名字不同，两个可以同时开）。

use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, IsIconic, SW_RESTORE, SetForegroundWindow, ShowWindow,
};
use windows::core::w;

/// 已有实例在跑就唤起它并返回 `false`；这是第一个实例返回 `true`。互斥体句柄故意不关，进程退出时由系统回收。
pub fn acquire() -> bool {
    let created = unsafe { CreateMutexW(None, false, w!("Local\\QingjianSettings")) };
    if created.is_err() || unsafe { GetLastError() } != ERROR_ALREADY_EXISTS {
        return true;
    }
    // 第一个实例还没建出窗口（连点两下）就找不到，直接退出即可
    if let Ok(hwnd) = unsafe { FindWindowW(None, w!("青简设置")) } {
        unsafe {
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            let _ = SetForegroundWindow(hwnd);
        }
    }
    false
}
