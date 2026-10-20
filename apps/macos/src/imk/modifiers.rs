//! 修饰键硬件状态与进程级中英文逻辑模式；所有入口都在主线程使用。

use std::cell::RefCell;

use super::mode_switch::ModeSwitch;
use objc2_app_kit::{NSEvent, NSEventModifierFlags};
use qingjian_platform::SwitchKeys;

/// Caps Lock 的硬件状态；用于变化同步和还原 AppKit 字母大小写。
pub fn caps_lock_on() -> bool {
    NSEvent::modifierFlags_class().contains(NSEventModifierFlags::CapsLock)
}

/// Shift 正按着。读的是此刻的硬件状态而不是事件自带的标志，但 Shift 是按住不放的键，处理按键时它几乎总还按着。
/// macOS 上 Caps Lock 亮着时按住 Shift 送来的仍是大写（不像 Windows 会反转），所以英文模式的大小写只能靠它判断。
pub fn shift_down() -> bool {
    NSEvent::modifierFlags_class().contains(NSEventModifierFlags::Shift)
}

thread_local! {
    static MODE: RefCell<ModeSwitch> = RefCell::new(ModeSwitch::default());
}

/// 逻辑中英文状态不等同于 Caps Lock 指示灯，也不等同于英文候选开关。
pub fn english_mode() -> bool {
    MODE.with(|mode| mode.borrow().english())
}

pub fn configure(enabled: bool, keys: SwitchKeys) {
    MODE.with(|mode| mode.borrow_mut().configure(enabled, keys));
    tracing::info!(enabled, switch_keys = %keys.describe(), "应用中英文切换配置");
}

pub fn sync_caps() -> bool {
    MODE.with(|mode| mode.borrow_mut().sync_caps(caps_lock_on()))
}

pub fn cancel_tap(reason: &'static str) {
    let flags = NSEvent::modifierFlags_class().bits();
    MODE.with(|mode| {
        let mut mode = mode.borrow_mut();
        if let Some(pending) = mode.pending() {
            tracing::info!(reason, pending, "取消修饰键单击判定");
        }
        mode.cancel_with_flags(flags);
    });
}

pub fn flags_changed(key: u16, flags: usize) -> bool {
    MODE.with(|mode| {
        let mut mode = mode.borrow_mut();
        let before = mode.english();
        let pending_before = mode.pending();
        let changed = mode.flags(key, flags);
        tracing::info!(
            key,
            flags = format_args!("{flags:#x}"),
            before,
            after = mode.english(),
            ?pending_before,
            pending_after = ?mode.pending(),
            changed,
            "收到修饰键事件"
        );
        changed
    })
}

pub fn key_down(key: u16, flags: usize, repeat: bool) -> bool {
    MODE.with(|mode| {
        let mut mode = mode.borrow_mut();
        if let Some(pending) = mode.pending() {
            tracing::info!(reason = "普通按键", pending, "取消修饰键单击判定");
        }
        mode.key_down(key, flags, repeat)
    })
}

/// 模式变更的组句清理可能在重入时借不到 Host；下次事件进入引擎前重试。
pub fn needs_finish() -> bool {
    MODE.with(|mode| mode.borrow().needs_finish())
}

pub fn did_finish() {
    MODE.with(|mode| mode.borrow_mut().did_finish());
}
