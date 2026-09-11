//! 按键分派用的虚拟键码与字符解析。功能键靠键码区分，字母 / 数字 / 标点用 `KeyEvent::character`。

use qingjian_platform::protocol::KeyEvent;

pub(super) const BACK: u32 = 0x08;
pub(super) const TAB: u32 = 0x09;
pub(super) const RETURN: u32 = 0x0D;
pub(super) const ESCAPE: u32 = 0x1B;
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

/// 敲出来是数字 1–9 的键（选候选用）：按 `character` 认，带 Shift 出的 `!` `@` 不算；没解析出字符时退回键码 0x31–0x39。
pub(super) fn digit(event: &KeyEvent) -> Option<usize> {
    match event.character {
        Some(c) => ('1'..='9').contains(&c).then(|| c as usize - '0' as usize),
        None => digit_key(event.virtual_key),
    }
}

/// 主键盘区数字键 1–9 的键码（0x31–0x39），不管修饰键：修饰键 + 数字的快捷键按键位认。
pub(super) fn digit_key(virtual_key: u32) -> Option<usize> {
    (0x31..=0x39)
        .contains(&virtual_key)
        .then(|| (virtual_key - 0x30) as usize)
}
