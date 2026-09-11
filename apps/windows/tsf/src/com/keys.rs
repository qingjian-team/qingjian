//! 把 TSF 送来的虚拟键码翻成协议的 [`KeyEvent`]，以及「组句中哪些键要吃」的判定。

use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, VIRTUAL_KEY, VK_BACK, VK_CONTROL, VK_ESCAPE, VK_LWIN, VK_MENU, VK_RETURN, VK_RWIN,
    VK_SHIFT, VK_SPACE, VK_TAB,
};

use qingjian_platform::protocol::{KeyEvent, KeyModifiers};

/// 采当前修饰键，解析出字符（字母恒小写、数字、空格、半角标点），Router 靠 `character` 分派。
pub(super) fn to_key_event(vk: u32) -> KeyEvent {
    let modifiers = current_modifiers();
    KeyEvent::new(vk, resolve_char(vk, modifiers.shift), modifiers)
}

/// A–Z。
pub(super) fn is_letter(vk: u32) -> bool {
    (0x41..=0x5A).contains(&vk)
}

/// 组句中要吃的功能键：退格 / Tab / 回车 / Esc / 空格 / 数字 0–9。
/// Tab 在没有整句补全时由 Router 判 Passthrough 交还应用。
pub(super) fn is_edit(vk: u32) -> bool {
    matches!(
        VIRTUAL_KEY(vk as u16),
        VK_BACK | VK_TAB | VK_RETURN | VK_ESCAPE | VK_SPACE
    ) || is_digit(vk)
}

/// 组句中要吃的方向 / 翻页键：PageUp/Down、End、Home、← ↑ → ↓（`0x21..=0x28`）。
pub(super) fn is_nav(vk: u32) -> bool {
    (0x21..=0x28).contains(&vk)
}

fn is_digit(vk: u32) -> bool {
    (0x30..=0x39).contains(&vk)
}

fn current_modifiers() -> KeyModifiers {
    KeyModifiers {
        ctrl: key_down(VK_CONTROL),
        shift: key_down(VK_SHIFT),
        alt: key_down(VK_MENU),
        win: key_down(VK_LWIN) || key_down(VK_RWIN),
    }
}

/// `GetKeyState` 高位为 1（返回值为负）表示按下。
fn key_down(vk: VIRTUAL_KEY) -> bool {
    // SAFETY: 只读当前线程的键状态。
    let state = unsafe { GetKeyState(vk.0 as i32) };
    state < 0
}

/// 虚拟键 + Shift → 字符（US 布局）。功能键返回 `None`。
fn resolve_char(vk: u32, shift: bool) -> Option<char> {
    if is_letter(vk) {
        return Some((b'a' + (vk - 0x41) as u8) as char);
    }
    if is_digit(vk) {
        let digit = (vk - 0x30) as usize;
        return Some(if shift {
            b")!@#$%^&*("[digit] as char
        } else {
            (b'0' + digit as u8) as char
        });
    }
    if VIRTUAL_KEY(vk as u16) == VK_SPACE {
        return Some(' ');
    }
    let (plain, shifted) = match vk {
        0xBA => (';', ':'),
        0xBB => ('=', '+'),
        0xBC => (',', '<'),
        0xBD => ('-', '_'),
        0xBE => ('.', '>'),
        0xBF => ('/', '?'),
        0xC0 => ('`', '~'),
        0xDB => ('[', '{'),
        0xDC => ('\\', '|'),
        0xDD => (']', '}'),
        0xDE => ('\'', '"'),
        _ => return None,
    };
    Some(if shift { shifted } else { plain })
}
