//! 呼出面板的全局快捷键：Carbon 的 `RegisterEventHotKey`，不要辅助功能权限，按下时在主线程的事件循环里回调。
//! 只在主线程调。

use std::cell::Cell;
use std::ffi::c_void;
use std::ptr;

use qingjian_clipboard::Hotkey;

use super::ffi::{
    CMD_KEY, CONTROL_KEY, EVENT_CLASS_KEYBOARD, EVENT_HOT_KEY_PRESSED, EventHotKeyId,
    EventTypeSpec, GetApplicationEventTarget, InstallEventHandler, OPTION_KEY, RegisterEventHotKey,
    SHIFT_KEY, UnregisterEventHotKey,
};
use super::keycode::key_code;

/// 注册时的签名（'QJCB'），按下时原样带回。
const SIGNATURE: u32 = u32::from_be_bytes(*b"QJCB");

thread_local! {
    /// 已注册的快捷键；换键时先注销它。
    static REGISTERED: Cell<*mut c_void> = const { Cell::new(ptr::null_mut()) };

    /// 收快捷键的处理函数装过没有：进程里装一次。
    static HANDLER_INSTALLED: Cell<bool> = const { Cell::new(false) };
}

extern "C" fn pressed(_call: *mut c_void, _event: *mut c_void, _data: *mut c_void) -> i32 {
    super::listener::hotkey_pressed();
    0
}

fn install_handler() {
    if HANDLER_INSTALLED.get() {
        return;
    }
    let types = [EventTypeSpec {
        event_class: EVENT_CLASS_KEYBOARD,
        event_kind: EVENT_HOT_KEY_PRESSED,
    }];
    // SAFETY: 处理函数是 'static 的 extern "C" 函数，事件类型数组在调用期间有效；Carbon 会复制它。
    let status = unsafe {
        InstallEventHandler(
            GetApplicationEventTarget(),
            pressed,
            types.len() as u32,
            types.as_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    if status == 0 {
        HANDLER_INSTALLED.set(true);
    } else {
        tracing::warn!(status, "cannot install the hotkey handler");
    }
}

/// 换成这个快捷键。被别的程序占着不算致命：照样记录，换个键再注册。
pub fn register(hotkey: Hotkey) {
    install_handler();
    let previous = REGISTERED.replace(ptr::null_mut());
    if !previous.is_null() {
        // SAFETY: 是之前注册成功拿到的句柄，只注销一次。
        unsafe { UnregisterEventHotKey(previous) };
    }
    let Some(code) = key_code(hotkey.key) else {
        tracing::warn!(?hotkey, "clipboard hotkey has no key code");
        return;
    };
    let mut modifiers = 0;
    for (held, modifier) in [
        (hotkey.win, CMD_KEY),
        (hotkey.shift, SHIFT_KEY),
        (hotkey.alt, OPTION_KEY),
        (hotkey.control, CONTROL_KEY),
    ] {
        if held {
            modifiers |= modifier;
        }
    }
    let mut registered = ptr::null_mut();
    // SAFETY: 输出指针指向本地变量；目标是本程序的事件目标。
    let status = unsafe {
        RegisterEventHotKey(
            u32::from(code),
            modifiers,
            EventHotKeyId {
                signature: SIGNATURE,
                id: 1,
            },
            GetApplicationEventTarget(),
            0,
            &mut registered,
        )
    };
    if status == 0 {
        REGISTERED.set(registered);
    } else {
        tracing::warn!(status, ?hotkey, "clipboard hotkey is taken");
    }
}
