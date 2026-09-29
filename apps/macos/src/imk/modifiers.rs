//! 当前修饰键状态。IMK 的 `inputText:client:` 不带事件对象，Caps Lock / Shift 只能从系统当前状态读。

use objc2_app_kit::{NSEvent, NSEventModifierFlags};

/// Caps Lock 亮着：视为英文模式，字母默认小写、按住 Shift 才大写、标点不转全角。
pub fn caps_lock_on() -> bool {
    NSEvent::modifierFlags_class().contains(NSEventModifierFlags::CapsLock)
}

/// Shift 正按着。读的是此刻的硬件状态而不是事件自带的标志，但 Shift 是按住不放的键，处理按键时它几乎总还按着。
/// macOS 上 Caps Lock 亮着时按住 Shift 送来的仍是大写（不像 Windows 会反转），所以英文模式的大小写只能靠它判断。
pub fn shift_down() -> bool {
    NSEvent::modifierFlags_class().contains(NSEventModifierFlags::Shift)
}

/// 此刻的修饰键状态（⇧ / ⌃ / ⌥ / ⌘），切换键单击判定用它做差量的初值。
pub fn current() -> qingjian_platform::Modifiers {
    let flags = NSEvent::modifierFlags_class();
    qingjian_platform::Modifiers {
        option: flags.contains(NSEventModifierFlags::Option),
        shift: flags.contains(NSEventModifierFlags::Shift),
        control: flags.contains(NSEventModifierFlags::Control),
        command: flags.contains(NSEventModifierFlags::Command),
    }
}
