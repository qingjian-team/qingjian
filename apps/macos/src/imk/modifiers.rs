//! 系统修饰键与输入法自己的中英文状态；供按键处理和菜单栏共同使用。

use std::cell::RefCell;

use super::mode::InputMode;

use objc2_app_kit::{NSEvent, NSEventModifierFlags};

thread_local! {
    static MODE: RefCell<InputMode> = RefCell::new(InputMode::default());
}

/// Caps Lock 的物理状态；不能用它直接判断快捷键切换后的中英文模式。
pub fn caps_lock_on() -> bool {
    NSEvent::modifierFlags_class().contains(NSEventModifierFlags::CapsLock)
}

/// 当前中英文模式，同时反映 Caps Lock 与可配置组合键的切换。
pub fn english_mode() -> bool {
    MODE.with_borrow(|mode| mode.english(caps_lock_on()))
}

/// 只切换输入法内部状态，不合成按键，不改变系统 Caps Lock。
pub fn toggle_mode() {
    MODE.with_borrow_mut(InputMode::toggle);
}
