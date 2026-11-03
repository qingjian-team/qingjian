//! objc2 没有封装的几组 C 接口：Carbon 的全局快捷键、Core Graphics 的按键状态与模拟按键、辅助功能权限。

use std::ffi::c_void;

/// `EventHotKeyID`：注册时给的标识，按下时原样带回来。
#[repr(C)]
#[derive(Clone, Copy)]
pub struct EventHotKeyId {
    pub signature: u32,
    pub id: u32,
}

/// `EventTypeSpec`。
#[repr(C)]
pub struct EventTypeSpec {
    pub event_class: u32,
    pub event_kind: u32,
}

pub type EventHandler = extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> i32;

/// `kEventClassKeyboard`（'keyb'）。
pub const EVENT_CLASS_KEYBOARD: u32 = u32::from_be_bytes(*b"keyb");

/// `kEventHotKeyPressed`。
pub const EVENT_HOT_KEY_PRESSED: u32 = 5;

/// Carbon 的修饰键位：`cmdKey`、`shiftKey`、`optionKey`、`controlKey`。
pub const CMD_KEY: u32 = 1 << 8;
pub const SHIFT_KEY: u32 = 1 << 9;
pub const OPTION_KEY: u32 = 1 << 11;
pub const CONTROL_KEY: u32 = 1 << 12;

/// `kCGEventSourceStateCombinedSessionState`：这个登录会话里所有键盘合起来的状态。
pub const COMBINED_SESSION_STATE: i32 = 0;

/// `kCGHIDEventTap`：从最底层发出，跟真按键走同一条路。
pub const HID_EVENT_TAP: u32 = 0;

/// `kCGEventFlagMaskCommand`。
pub const FLAG_COMMAND: u64 = 1 << 20;

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    pub fn GetApplicationEventTarget() -> *mut c_void;

    pub fn InstallEventHandler(
        target: *mut c_void,
        handler: EventHandler,
        count: u32,
        types: *const EventTypeSpec,
        user_data: *mut c_void,
        out: *mut *mut c_void,
    ) -> i32;

    pub fn RegisterEventHotKey(
        key_code: u32,
        modifiers: u32,
        id: EventHotKeyId,
        target: *mut c_void,
        options: u32,
        out: *mut *mut c_void,
    ) -> i32;

    pub fn UnregisterEventHotKey(hotkey: *mut c_void) -> i32;
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    pub fn CGEventSourceKeyState(state: i32, key: u16) -> bool;

    pub fn CGEventCreateKeyboardEvent(source: *mut c_void, key: u16, down: bool) -> *mut c_void;

    pub fn CGEventSetFlags(event: *mut c_void, flags: u64);

    pub fn CGEventPost(tap: u32, event: *mut c_void);
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    pub fn CFRelease(object: *const c_void);
}

#[link(name = "SystemConfiguration", kind = "framework")]
unsafe extern "C" {
    /// 「系统设置 → 通用 → 共享」里的电脑名；返回的 CFString 归调用方释放。
    pub fn SCDynamicStoreCopyComputerName(
        store: *const c_void,
        encoding: *mut u32,
    ) -> *const c_void;
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    pub fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;

    pub static kAXTrustedCheckOptionPrompt: *const c_void;
}
