//! X11 keysym：常量与「这个 keysym 敲出来是哪个字符」。
//!
//! IBus 的 `ProcessKeyEvent` 直接给 keysym。取值来自 X11 的 `keysymdef.h`，
//! 是三十年没动过的公开常量，不值得为它引一个依赖。
//!
//! 这里只列 [`super::mapping`] 用得到的那些：功能键区、小键盘、两个 Shift。

/// 功能键区（`0xff00` 起）。
pub const BACKSPACE: u32 = 0xff08;
pub const TAB: u32 = 0xff09;
pub const RETURN: u32 = 0xff0d;
pub const ESCAPE: u32 = 0xff1b;
pub const HOME: u32 = 0xff50;
pub const LEFT: u32 = 0xff51;
pub const UP: u32 = 0xff52;
pub const RIGHT: u32 = 0xff53;
pub const DOWN: u32 = 0xff54;
pub const PAGE_UP: u32 = 0xff55;
pub const PAGE_DOWN: u32 = 0xff56;
pub const END: u32 = 0xff57;
pub const DELETE: u32 = 0xffff;

/// Shift + Tab：X11 不是「Tab 加 Shift 修饰键」，而是另一个 keysym。
pub const ISO_LEFT_TAB: u32 = 0xfe20;

/// 两个 Shift 键本身：中 / 英切换要认它的按下与抬起（`[shortcut] switch_mode`）。
pub const SHIFT_L: u32 = 0xffe1;
pub const SHIFT_R: u32 = 0xffe2;

/// 小键盘：NumLock 灭着时方向 / 编辑键走这一段，亮着时 `KP_0`–`KP_9` 出数字。
pub const KP_ENTER: u32 = 0xff8d;
pub const KP_HOME: u32 = 0xff95;
pub const KP_LEFT: u32 = 0xff96;
pub const KP_UP: u32 = 0xff97;
pub const KP_RIGHT: u32 = 0xff98;
pub const KP_DOWN: u32 = 0xff99;
pub const KP_PAGE_UP: u32 = 0xff9a;
pub const KP_PAGE_DOWN: u32 = 0xff9b;
pub const KP_END: u32 = 0xff9c;
pub const KP_BEGIN: u32 = 0xff9d;
pub const KP_INSERT: u32 = 0xff9e;
pub const KP_DELETE: u32 = 0xff9f;

/// 小键盘的运算符键。
pub const KP_MULTIPLY: u32 = 0xffaa;
pub const KP_ADD: u32 = 0xffab;
pub const KP_SEPARATOR: u32 = 0xffac;
pub const KP_SUBTRACT: u32 = 0xffad;
pub const KP_DECIMAL: u32 = 0xffae;
pub const KP_DIVIDE: u32 = 0xffaf;

/// 小键盘数字段。
pub const KP_0: u32 = 0xffb0;
pub const KP_9: u32 = 0xffb9;

/// Unicode keysym 的前缀：`0x01000000 | 码点`。
const UNICODE_BASE: u32 = 0x0100_0000;

/// Unicode keysym 能表示的最大码点。
const UNICODE_MAX: u32 = 0x0010_ffff;

/// 这个 keysym 敲出来是哪个可见字符；功能键与修饰键没有字符，是 `None`。
///
/// 三段来源：Latin-1 段（keysym 就是码点）、Unicode 段（`0x01000000 | 码点`）、小键盘数字。
/// 控制字符不返回——Server 那边 `KeyEvent::character` 只认可见字符，功能键靠 `virtual_key`。
pub fn to_char(keysym: u32) -> Option<char> {
    if (KP_0..=KP_9).contains(&keysym) {
        return char::from_u32('0' as u32 + (keysym - KP_0));
    }
    // Latin-1：可打印的两段，中间的 0x7f–0x9f 是控制字符
    if (0x20..=0x7e).contains(&keysym) || (0xa0..=0xff).contains(&keysym) {
        return char::from_u32(keysym);
    }
    if (UNICODE_BASE..=UNICODE_BASE + UNICODE_MAX).contains(&keysym) {
        return char::from_u32(keysym - UNICODE_BASE).filter(|c| !c.is_control());
    }
    None
}
