//! keysym + 修饰键掩码 → 协议里的 [`KeyEvent`]。
//!
//! **这张表必须与 Fcitx5 插件的 `apps/linux/fcitx5/src/key/mapping.cpp` 一字不差**：
//! 两个前端喂的是同一个 Server、同一套 `[shortcut]` 配置，映射不一致的话同一个键在两个框架下
//! 行为就不一样（最容易出问题的是翻页键与 Shift+数字）。改这里要同时改那边。
//!
//! 虚拟键码用 Windows 那套值（`0x08` 退格、`0x25` 左…），来源见
//! [`qingjian_platform::protocol::KeyEvent`]：Server 的按键分流是照它写的，Linux 这边照抄。

use qingjian_platform::protocol::KeyEvent;

use super::{keysym, modifiers};

/// Shift + 数字行敲出来的符号，按数字 0–9 排。
///
/// 译词与删候选的快捷键按**物理数字行**认（`Shift+2` 是第 2 条），而 X11 给的 keysym 是 `@`。
/// 所以这里把它折回 `0x32`，`character` 仍保留 `@` 不动——上屏的还是符号本身。
const SHIFTED_DIGITS: [char; 10] = [')', '!', '@', '#', '$', '%', '^', '&', '*', '('];

/// 把 IBus 给的一次按键翻成 [`KeyEvent`]。
///
/// `state` 是 X11 的修饰键掩码（IBus 的 `IBusModifierType` 与它同值）。
/// `english_mode` 由调用方按当前中 / 英状态补，这一层不知道输入法的模式。
pub fn to_key_event(keysym_value: u32, state: u32) -> KeyEvent {
    let mut modifiers = modifiers::from_mask(state);
    // ISO_Left_Tab 是「Shift+Tab」那一个 keysym，掩码里不一定带 Shift 位，补上
    if keysym_value == keysym::ISO_LEFT_TAB {
        modifiers.shift = true;
    }
    KeyEvent {
        virtual_key: virtual_key(keysym_value, modifiers.shift),
        character: keysym::to_char(keysym_value),
        modifiers,
    }
}

/// keysym → 虚拟键码。认不出来的原样交出去（ASCII 字母与符号就走这一条）。
fn virtual_key(keysym_value: u32, shift: bool) -> u32 {
    // Shift+数字先判：keysym 是符号，要的却是数字行的键码
    if keysym_value < 128
        && shift
        && let Some(character) = char::from_u32(keysym_value)
        && let Some(digit) = SHIFTED_DIGITS.iter().position(|c| *c == character)
    {
        return 0x30 + digit as u32;
    }
    if (keysym::KP_0..=keysym::KP_9).contains(&keysym_value) {
        return 0x60 + (keysym_value - keysym::KP_0);
    }
    match keysym_value {
        keysym::BACKSPACE => 0x08,
        keysym::TAB | keysym::ISO_LEFT_TAB => 0x09,
        keysym::RETURN | keysym::KP_ENTER => 0x0d,
        keysym::KP_BEGIN => 0x0c,
        keysym::ESCAPE => 0x1b,
        keysym::SHIFT_L | keysym::SHIFT_R => 0x10,
        keysym::PAGE_UP | keysym::KP_PAGE_UP => 0x21,
        keysym::PAGE_DOWN | keysym::KP_PAGE_DOWN => 0x22,
        keysym::END | keysym::KP_END => 0x23,
        keysym::HOME | keysym::KP_HOME => 0x24,
        keysym::LEFT | keysym::KP_LEFT => 0x25,
        keysym::UP | keysym::KP_UP => 0x26,
        keysym::RIGHT | keysym::KP_RIGHT => 0x27,
        keysym::DOWN | keysym::KP_DOWN => 0x28,
        keysym::KP_INSERT => 0x2d,
        keysym::DELETE | keysym::KP_DELETE => 0x2e,
        keysym::KP_MULTIPLY => 0x6a,
        keysym::KP_ADD => 0x6b,
        keysym::KP_SEPARATOR => 0x6c,
        keysym::KP_SUBTRACT => 0x6d,
        keysym::KP_DECIMAL => 0x6e,
        keysym::KP_DIVIDE => 0x6f,
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// X11 的 Shift 位。
    const SHIFT: u32 = 1 << 0;

    /// 字母原样过去：keysym 就是键码，字符跟着。
    #[test]
    fn letters_pass_through() {
        let event = to_key_event('n' as u32, 0);
        assert_eq!(event.virtual_key, 'n' as u32);
        assert_eq!(event.character, Some('n'));
        assert!(!event.modifiers.shift);
    }

    /// Shift+2：键码折回数字行的 `0x32`，字符仍是 `@`（这是 Linux 上真机踩过的坑）。
    #[test]
    fn shifted_digits_fold_back_to_the_number_row() {
        let event = to_key_event('@' as u32, SHIFT);
        assert_eq!(event.virtual_key, 0x32);
        assert_eq!(event.character, Some('@'));
        assert!(event.modifiers.shift);
    }

    /// 没按 Shift 的 `@`（有的布局上直接就有）不折。
    #[test]
    fn unshifted_symbol_is_not_folded() {
        assert_eq!(to_key_event('@' as u32, 0).virtual_key, '@' as u32);
    }

    /// 功能键与小键盘按 Windows 那套键码走。
    #[test]
    fn function_and_keypad_keys_use_windows_codes() {
        assert_eq!(to_key_event(keysym::BACKSPACE, 0).virtual_key, 0x08);
        assert_eq!(to_key_event(keysym::LEFT, 0).virtual_key, 0x25);
        assert_eq!(to_key_event(keysym::KP_LEFT, 0).virtual_key, 0x25);
        assert_eq!(to_key_event(keysym::KP_ENTER, 0).virtual_key, 0x0d);
        assert_eq!(to_key_event(keysym::KP_0, 0).virtual_key, 0x60);
        assert_eq!(to_key_event(keysym::KP_9, 0).virtual_key, 0x69);
        assert_eq!(to_key_event(keysym::SHIFT_L, 0).virtual_key, 0x10);
        // 小键盘数字有字符，方向键没有
        assert_eq!(to_key_event(keysym::KP_0, 0).character, Some('0'));
        assert_eq!(to_key_event(keysym::LEFT, 0).character, None);
    }

    /// Shift+Tab 是另一个 keysym，掩码里没 Shift 也要认成带 Shift 的 Tab。
    #[test]
    fn iso_left_tab_is_shift_tab() {
        let event = to_key_event(keysym::ISO_LEFT_TAB, 0);
        assert_eq!(event.virtual_key, 0x09);
        assert!(event.modifiers.shift);
    }
}
