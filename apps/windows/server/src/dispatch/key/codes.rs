//! 按键分派用的虚拟键码与字符解析。

use qingjian_platform::MAX_PAGE_SIZE;
use qingjian_platform::protocol::KeyEvent;

use crate::dispatch::Router;

impl Router {
    pub(super) fn page_step(&self, event: &KeyEvent) -> Option<isize> {
        page_key(event, self.config.page_keys)
            .or_else(|| self.config.extra_page_keys.step(event.character?))
    }
}

pub(crate) const BACK: u32 = 0x08;
pub(crate) const TAB: u32 = 0x09;
pub(crate) const RETURN: u32 = 0x0D;
pub(crate) const ESCAPE: u32 = 0x1B;
pub(crate) const PRIOR: u32 = 0x21;
pub(crate) const NEXT: u32 = 0x22;
pub(crate) const END: u32 = 0x23;
pub(crate) const HOME: u32 = 0x24;
pub(crate) const LEFT: u32 = 0x25;
pub(crate) const UP: u32 = 0x26;
pub(crate) const RIGHT: u32 = 0x27;
pub(crate) const DOWN: u32 = 0x28;
pub(crate) const DELETE: u32 = 0x2E;
pub(crate) const KEY_A: u32 = 0x41;

/// 翻页键对 `(上一页, 下一页)`：返回 -1 / +1。
pub(crate) fn page_key(event: &KeyEvent, page_keys: (char, char)) -> Option<isize> {
    let c = event.character?;
    if c == page_keys.0 {
        Some(-1)
    } else if c == page_keys.1 {
        Some(1)
    } else {
        None
    }
}

/// 敲出来是数字的键（选候选用）：按 `character` 认，Shift 出的 `!@#` 不算。
/// `1`–`9` 对 1–9，`0` 对第 10 个。
pub(crate) fn digit(event: &KeyEvent) -> Option<usize> {
    match event.character {
        Some(c) => digit_of(c),
        None => digit_key(event.virtual_key),
    }
}

/// 数字字符对应的候选位次：`0` 是第 10 个，与 [`MAX_PAGE_SIZE`] 对齐。
fn digit_of(c: char) -> Option<usize> {
    match c {
        '1'..='9' => Some(c as usize - '0' as usize),
        '0' => Some(MAX_PAGE_SIZE),
        _ => None,
    }
}

/// 主键盘区数字键的键码，不管修饰键（修饰键 + 数字的快捷键按键位认）。
pub(crate) fn digit_key(virtual_key: u32) -> Option<usize> {
    match virtual_key {
        0x31..=0x39 => Some((virtual_key - 0x30) as usize),
        0x30 => Some(MAX_PAGE_SIZE),
        _ => None,
    }
}

/// 小键盘区的键（数字与 `* + - . /`）：敲出来的标点一律半角。
pub(crate) fn is_keypad(virtual_key: u32) -> bool {
    (0x60..=0x6F).contains(&virtual_key)
}
