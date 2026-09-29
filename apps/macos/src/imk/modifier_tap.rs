//! 修饰键 flagsChanged 的 CGEventTap。
//!
//! IMK 的 `handleEvent` 不收修饰键（Shift / Ctrl）的按下 / 抬起事件，单击切换键只能从这里判：
//! 建一个 `ListenOnly` 的 tap 监听 `kCGEventFlagsChanged`，读出当前修饰键状态交给
//! [`crate::host::Host::handle_modifier_change`] 判单击。组合键 Ctrl + Alt + Space 走 IMK 的空格键
//! 事件、Caps Lock 走硬件状态轮询，都不经过这里。
//!
//! 需要辅助功能权限（系统设置 → 隐私与安全性 → 辅助功能）；没给权限 tap 建不出来，单击 Shift / Ctrl
//! 失效，组合键与 Caps Lock 仍可用。

use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};

use objc2_core_foundation::{
    CFMachPort, CFRetained, CFRunLoop, CFRunLoopSource, kCFRunLoopCommonModes,
};
use objc2_core_graphics::{
    CGEvent, CGEventFlags, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventTapProxy, CGEventType,
};
use qingjian_platform::Modifiers;

// 主线程持有 tap 与它的 run loop source；活着 tap 才有效，掉一个就被释放、监听停掉。
thread_local! {
    static TAP: RefCell<Option<(CFRetained<CFMachPort>, CFRetained<CFRunLoopSource>)>> =
        const { RefCell::new(None) };
}

/// 权限没给的警告只记一次：每次激活都会调 `start`，别刷屏。
static PERMISSION_WARNED: AtomicBool = AtomicBool::new(false);

/// flagsChanged 回调：读出当前修饰键状态交给 Host 判切换键单击。事件原样放行（`ListenOnly`）。
///
/// # Safety
/// 系统保证 `event` 在回调期间有效；`user_info` 传的是空指针，不解引用。
unsafe extern "C-unwind" fn flags_changed_callback(
    _proxy: CGEventTapProxy,
    event_type: CGEventType,
    event: NonNull<CGEvent>,
    _user_info: *mut c_void,
) -> *mut CGEvent {
    let event_ptr = event.as_ptr();
    if event_type == CGEventType::FlagsChanged {
        // SAFETY: 系统送来的有效事件指针
        let flags = unsafe { CGEvent::flags(Some(&*event_ptr)) };
        crate::host::with(|h| h.handle_modifier_change(modifiers_from_cg_flags(flags)));
    }
    event_ptr
}

/// CGEventFlags 到 [`Modifiers`]：⌥ 是 Alternate、⇧ 是 Shift、⌃ 是 Control、⌘ 是 Command。
fn modifiers_from_cg_flags(flags: CGEventFlags) -> Modifiers {
    Modifiers {
        option: flags.contains(CGEventFlags::MaskAlternate),
        shift: flags.contains(CGEventFlags::MaskShift),
        control: flags.contains(CGEventFlags::MaskControl),
        command: flags.contains(CGEventFlags::MaskCommand),
    }
}

/// 建 tap 并加进主 run loop，必须在主线程调（run loop 在跑才收得到事件）。
/// 幂等：建过就返回 `true`。返回 `false` 是权限没给或建不出来——单击切换键失效，调用方提示用户。
pub fn start() -> bool {
    TAP.with(|cell| start_inner(&mut cell.borrow_mut()))
}

fn start_inner(tap: &mut Option<(CFRetained<CFMachPort>, CFRetained<CFRunLoopSource>)>) -> bool {
    if tap.is_some() {
        return true;
    }
    if !ax_is_process_trusted() {
        warn_permission_once();
        return false;
    }
    // 只对 flagsChanged 感兴趣（事件类型 12，对应位 12）
    let mask = 1u64 << CGEventType::FlagsChanged.0;
    // SAFETY: 回调按 CGEventTapCallBack 的约定实现，user_info 传空指针
    let created = unsafe {
        CGEvent::tap_create(
            CGEventTapLocation::SessionEventTap,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::ListenOnly,
            mask,
            Some(flags_changed_callback),
            std::ptr::null_mut(),
        )
    };
    let Some(port) = created else {
        tracing::warn!("CGEventTap 创建失败：辅助功能权限没给或被系统拒了，单击切换键失效");
        return false;
    };
    let Some(source) = CFMachPort::new_run_loop_source(None, Some(&port), 0) else {
        tracing::warn!("CGEventTap 的 run loop source 创建失败");
        return false;
    };
    if let Some(run_loop) = CFRunLoop::main() {
        // SAFETY: kCFRunLoopCommonModes 是系统只读常量
        unsafe { run_loop.add_source(Some(&source), kCFRunLoopCommonModes) };
    }
    CGEvent::tap_enable(&port, true);
    *tap = Some((port, source));
    tracing::info!("修饰键 flagsChanged 监听已启用：单击 Shift / Ctrl 切中英生效");
    true
}

/// 辅助功能权限是否已授予（ApplicationServices 的 `AXIsProcessTrusted`）。
fn ax_is_process_trusted() -> bool {
    unsafe extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }
    // SAFETY: 无参无指针的系统函数
    unsafe { AXIsProcessTrusted() }
}

/// 权限没给的警告只记一次。
fn warn_permission_once() {
    if PERMISSION_WARNED.swap(true, Ordering::SeqCst) {
        return;
    }
    tracing::warn!(
        "辅助功能权限未授予：单击 Shift / Ctrl 切中英失效，组合键 Ctrl + Alt + Space 与 Caps Lock 仍可用"
    );
}
