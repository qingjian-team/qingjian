//! 粘贴回原窗口：写剪贴板，等面板藏好后把原窗口切回前台，再模拟 Ctrl+V。
//! 目标窗口以管理员身份运行时系统会拦下模拟按键（UIPI），这种情况内容仍留在剪贴板上。

use std::time::Duration;

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput,
    VIRTUAL_KEY, VK_CONTROL, VK_V,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SetForegroundWindow};

use super::clipboard::write_payload;
use crate::system::Payload;

/// 等面板藏好、系统把焦点让出来。
const HIDE_DELAY: Duration = Duration::from_millis(80);

/// 等原窗口拿到键盘焦点。到了前台不等于已经能收键。
const FOCUS_DELAY: Duration = Duration::from_millis(40);

/// 切前台最多试几次、隔多久：面板还没藏好时系统会拒绝，只试一次粘贴就悄悄丢了。
const FRONT_ATTEMPTS: u32 = 25;

const FRONT_RETRY: Duration = Duration::from_millis(20);

fn bring_to_front(target: isize) -> bool {
    let window = HWND(target as *mut _);
    for attempt in 0..FRONT_ATTEMPTS {
        if unsafe { SetForegroundWindow(window) }.as_bool()
            && unsafe { GetForegroundWindow() } == window
        {
            return true;
        }
        if attempt + 1 < FRONT_ATTEMPTS {
            std::thread::sleep(FRONT_RETRY);
        }
    }
    tracing::warn!("cannot bring the previous window to the foreground");
    false
}

pub fn paste(payload: Payload, target: isize) {
    std::thread::spawn(move || {
        write_payload(&payload);
        std::thread::sleep(HIDE_DELAY);
        if !bring_to_front(target) {
            return;
        }
        std::thread::sleep(FOCUS_DELAY);
        let keys = [
            key(VK_CONTROL, false),
            key(VK_V, false),
            key(VK_V, true),
            key(VK_CONTROL, true),
        ];
        let sent = unsafe { SendInput(&keys, size_of::<INPUT>() as i32) };
        if sent as usize != keys.len() {
            tracing::warn!(sent, "simulated paste was blocked");
        }
    });
}

/// 只放上剪贴板，不粘贴；焦点还给原窗口。
pub fn copy(payload: Payload, target: isize) {
    std::thread::spawn(move || {
        write_payload(&payload);
        std::thread::sleep(HIDE_DELAY);
        bring_to_front(target);
    });
}

/// 面板藏好后把原窗口切回前台。
pub fn refocus(target: isize) {
    std::thread::spawn(move || {
        std::thread::sleep(HIDE_DELAY);
        bring_to_front(target);
    });
}

fn key(code: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: code,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                ..KEYBDINPUT::default()
            },
        },
    }
}
