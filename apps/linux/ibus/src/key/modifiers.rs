//! 修饰键掩码 → [`KeyModifiers`]。
//!
//! 复用协议里的 [`KeyModifiers`] 而不是另立一个：它的六个位（ctrl / shift / alt / win / caps /
//! english_mode）本来就是平台无关的，类型注释里的「Windows 语义」说的是字段怎么来的，不是字段本身。
//! 配置层 `Modifiers` 到它的映射（option→Alt、command→Super）在 Linux 上同样成立。

//!
//! 掩码取值是 X11 的修饰键位，IBus（`IBusModifierType`）与 GDK 用的是同一套。

use qingjian_platform::protocol::KeyModifiers;

/// Shift。
const SHIFT: u32 = 1 << 0;

/// Caps Lock 锁定位（X11 叫 Lock）。
const LOCK: u32 = 1 << 1;

/// Control。
const CONTROL: u32 = 1 << 2;

/// Mod1，常规键盘布局上是 Alt。
const MOD1: u32 = 1 << 3;

/// Mod4，常规键盘布局上是 Super（⊞ / ⌘ 那个键）。
const MOD4: u32 = 1 << 6;

/// 抬键事件的标志位：IBus 把按下与抬起走同一个回调，靠它区分。
const RELEASE: u32 = 1 << 30;

/// 掩码里的物理修饰键与 Caps Lock 位。`english_mode` 是输入法自己的状态，不在掩码里，
/// 由壳在外面用 [`KeyModifiers::english_mode`] 补上。
pub fn from_mask(state: u32) -> KeyModifiers {
    KeyModifiers {
        ctrl: state & CONTROL != 0,
        shift: state & SHIFT != 0,
        alt: state & MOD1 != 0,
        win: state & MOD4 != 0,
        caps: state & LOCK != 0,
        english_mode: false,
    }
}

/// 这是一次抬键（不是按下）。
///
/// 抬键**照样要报给 Server**：中 / 英切换认的是 Shift 的抬起。这里只负责说清是哪一种。
pub fn is_release(state: u32) -> bool {
    state & RELEASE != 0
}
