//! 粘贴回原来的程序：写剪贴板，等面板藏好后把原程序切回前台，再模拟 ⌘V。
//! 模拟按键要「辅助功能」权限：没给时第一次会弹系统的授权提示，这次内容仍留在剪贴板上，用户自己按 ⌘V 也行。

use std::ptr;
use std::time::Duration;

use objc2::rc::Retained;
use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication, NSWorkspace};
use objc2_foundation::{NSDictionary, NSNumber, NSString};

use super::ffi::{
    AXIsProcessTrustedWithOptions, CFRelease, CGEventCreateKeyboardEvent, CGEventPost,
    CGEventSetFlags, FLAG_COMMAND, HID_EVENT_TAP, kAXTrustedCheckOptionPrompt,
};
use super::keycode;
use super::pasteboard::write_payload;
use crate::system::Payload;

/// 等面板藏好、系统把焦点让出来。
const HIDE_DELAY: Duration = Duration::from_millis(80);

/// 等原程序拿到键盘焦点。到了前台不等于已经能收键。
const FOCUS_DELAY: Duration = Duration::from_millis(60);

/// 切前台最多试几次、隔多久：面板还没藏好时切不过去，只试一次粘贴就悄悄丢了。
const FRONT_ATTEMPTS: u32 = 25;

const FRONT_RETRY: Duration = Duration::from_millis(20);

fn bring_to_front(target: isize) -> bool {
    let Ok(pid) = i32::try_from(target) else {
        return false;
    };
    let Some(app): Option<Retained<NSRunningApplication>> =
        NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
    else {
        return false;
    };
    for attempt in 0..FRONT_ATTEMPTS {
        let front = NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .is_some_and(|front| front.processIdentifier() == pid);
        if front {
            return true;
        }
        #[allow(deprecated)]
        app.activateWithOptions(NSApplicationActivationOptions::ActivateIgnoringOtherApps);
        if attempt + 1 < FRONT_ATTEMPTS {
            std::thread::sleep(FRONT_RETRY);
        }
    }
    tracing::warn!("cannot bring the previous app to the front");
    false
}

/// 有没有辅助功能权限；没有时让系统弹授权提示（系统自己记着弹过没有，不会每次都弹）。
fn may_send_keys() -> bool {
    // SAFETY: 系统定义的常量字符串，与 NSString 免费桥接。
    let key: &NSString = unsafe { &*kAXTrustedCheckOptionPrompt.cast::<NSString>() };
    let options = NSDictionary::from_slices(&[key], &[&*NSNumber::new_bool(true)]);
    let options = Retained::as_ptr(&options).cast();
    // SAFETY: 字典在调用期间有效，键值都是 CF 能读的类型。
    unsafe { AXIsProcessTrustedWithOptions(options) }
}

fn send_command_v() {
    for down in [true, false] {
        // SAFETY: 不带事件源建按键事件，设上 ⌘ 后发出、再释放；建不出来时为空指针，跳过。
        unsafe {
            let event = CGEventCreateKeyboardEvent(ptr::null_mut(), keycode::V, down);
            if event.is_null() {
                tracing::warn!("cannot create the paste key event");
                return;
            }
            CGEventSetFlags(event, FLAG_COMMAND);
            CGEventPost(HID_EVENT_TAP, event);
            CFRelease(event);
        }
    }
}

pub fn paste(payload: Payload, target: isize) {
    std::thread::spawn(move || {
        write_payload(&payload);
        std::thread::sleep(HIDE_DELAY);
        if !bring_to_front(target) {
            return;
        }
        if !may_send_keys() {
            tracing::warn!("no accessibility permission, content left on the clipboard");
            return;
        }
        std::thread::sleep(FOCUS_DELAY);
        send_command_v();
    });
}

/// 只放上剪贴板，不粘贴；焦点还给原程序。
pub fn copy(payload: Payload, target: isize) {
    std::thread::spawn(move || {
        write_payload(&payload);
        std::thread::sleep(HIDE_DELAY);
        bring_to_front(target);
    });
}

/// 面板藏好后把原程序切回前台。
pub fn refocus(target: isize) {
    std::thread::spawn(move || {
        std::thread::sleep(HIDE_DELAY);
        bring_to_front(target);
    });
}
