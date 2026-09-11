//! 按键分派用的虚拟键码与字符解析。功能键靠键码区分，字母 / 数字 / 标点用 `KeyEvent::character`。

use qingjian_platform::protocol::KeyEvent;

pub(super) const BACK: u32 = 0x08;
pub(super) const TAB: u32 = 0x09;
pub(super) const RETURN: u32 = 0x0D;
pub(super) const ESCAPE: u32 = 0x1B;
pub(super) const SPACE: u32 = 0x20;
pub(super) const PRIOR: u32 = 0x21;
pub(super) const NEXT: u32 = 0x22;
pub(super) const END: u32 = 0x23;
pub(super) const HOME: u32 = 0x24;
pub(super) const LEFT: u32 = 0x25;
pub(super) const UP: u32 = 0x26;
pub(super) const RIGHT: u32 = 0x27;
pub(super) const DOWN: u32 = 0x28;

/// 翻页键对 `(上一页, 下一页)`（`[general] page_keys`）：返回 -1 / +1。
pub(super) fn page_key(event: &KeyEvent, page_keys: (char, char)) -> Option<isize> {
    let c = event.character?;
    if c == page_keys.0 {
        Some(-1)
    } else if c == page_keys.1 {
        Some(1)
    } else {
        None
    }
}

/// 数字键 1–9（`character` 优先，退回键码 0x31–0x39）。
pub(super) fn digit(event: &KeyEvent) -> Option<usize> {
    if let Some(c) = event.character.filter(|c| ('1'..='9').contains(c)) {
        return Some(c as usize - '0' as usize);
    }
    (0x31..=0x39)
        .contains(&event.virtual_key)
        .then(|| (event.virtual_key - 0x30) as usize)
}
