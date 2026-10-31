//! 单击中 / 英切换键的判定，喂的是击键 sink 的 `OnTestKeyDown` / `OnTestKeyUp`（被吃掉的键也经过它们，
//! 与 `WH_KEYBOARD` 钩子不同）。按下切换键到抬起之间没插进别的键，就是一次单击。
//!
//! 切换键来自 `[shortcut] switch_mode`，单击 Shift / 单击 Ctrl 可以都勾；Ctrl + Alt + Space 是组合键，走保留键。
//! 系统热键（如 Ctrl + Space）的第二个键被系统截走、到不了这里，看起来就像单击了 Ctrl：
//! 系统热键生效时调 [`KeyTap::cancel`] 作废这次按下。

use std::cell::Cell;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::LPARAM;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_CONTROL, VK_LCONTROL, VK_LSHIFT, VK_RCONTROL, VK_RSHIFT, VK_SHIFT,
};

use qingjian_platform::{SwitchKey, SwitchKeys};

#[derive(Default)]
pub(crate) struct KeyTap {
    /// 按下了哪个切换键、之后还没有别的键插进来。
    pressed: Cell<Option<SwitchKey>>,

    pending_shift: Cell<Option<Instant>>,
}

const DOUBLE_TAP_WINDOW: Duration = Duration::from_millis(400);

impl KeyTap {
    /// 任一键按下。`lparam` 第 30 位是按下前的状态（1 = 自动重复，不算新按下）。
    pub(crate) fn key_down(&self, vk: u32, lparam: LPARAM, keys: SwitchKeys) {
        let Some(key) = tap_key(keys, vk) else {
            self.cancel();
            return;
        };
        if (lparam.0 >> 30) & 1 != 0 {
            return;
        }
        // 两个切换键一起按（Ctrl + Shift 是系统换布局的键）不算单击
        let pressed = match self.pressed.get() {
            Some(other) if other != key => None,
            _ => Some(key),
        };
        self.pressed.set(pressed);
    }

    /// 任一键抬起；切换键单独抬起返回 `true`，一次抬起只算一次。
    pub(crate) fn key_up(&self, vk: u32, keys: SwitchKeys) -> bool {
        let Some(key) = tap_key(keys, vk) else {
            return false;
        };
        if self.pressed.get() == Some(key) {
            self.pressed.set(None);
            true
        } else {
            false
        }
    }

    /// 作废正按着的切换键（按下之后发生了别的事，这次抬起不算单击）。
    pub(crate) fn cancel(&self) {
        self.pressed.set(None);
    }

    /// Shift 单击进入待定；400 ms 内第二次返回 true（双击），第一下不立即切模式。
    pub(crate) fn double_shift(&self, enabled: bool) -> bool {
        if !enabled {
            self.pending_shift.set(None);
            return false;
        }
        let now = Instant::now();
        let double = self
            .pending_shift
            .get()
            .is_some_and(|at| now.duration_since(at) <= DOUBLE_TAP_WINDOW);
        self.pending_shift.set((!double).then_some(now));
        double
    }

    /// 待定 Shift 已超过 400 ms，取走并交给调用方按单击 Shift 配置处理。
    pub(crate) fn resolve_shift(&self) -> bool {
        if self.pressed.get() == Some(SwitchKey::Shift) {
            return false;
        }
        let now = Instant::now();
        if self
            .pending_shift
            .get()
            .is_none_or(|at| now.duration_since(at) < DOUBLE_TAP_WINDOW)
        {
            return false;
        }
        self.pending_shift.set(None);
        true
    }

    /// 用户在等待窗口内开始敲别的键：立即把待定 Shift 当单击落实。
    pub(crate) fn take_pending_shift(&self) -> bool {
        self.pending_shift.take().is_some()
    }
}

/// 这个虚拟键码是不是勾着的单击切换键（左右两个都算）；组合键 Ctrl + Alt + Space 不走单击判定。
fn tap_key(keys: SwitchKeys, vk: u32) -> Option<SwitchKey> {
    let is = |codes: [u16; 3]| codes.iter().any(|code| u32::from(*code) == vk);
    if keys.shift && is([VK_SHIFT.0, VK_LSHIFT.0, VK_RSHIFT.0]) {
        Some(SwitchKey::Shift)
    } else if keys.control && is([VK_CONTROL.0, VK_LCONTROL.0, VK_RCONTROL.0]) {
        Some(SwitchKey::Control)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 按下再抬起的 lparam（第 30 位为 0 表示新按下）。
    const DOWN: LPARAM = LPARAM(0);
    const VK_SHIFT_LEFT: u32 = 0xA0;
    const VK_CONTROL_LEFT: u32 = 0xA2;

    fn only(key: SwitchKey) -> SwitchKeys {
        SwitchKeys::NONE.with(key, true)
    }

    #[test]
    fn shift_tap_fires_only_when_nothing_else_interrupts() {
        let tap = KeyTap::default();
        let keys = only(SwitchKey::Shift);
        tap.key_down(VK_SHIFT_LEFT, DOWN, keys);
        assert!(tap.key_up(VK_SHIFT_LEFT, keys));
        // 一次抬起只算一次
        assert!(!tap.key_up(VK_SHIFT_LEFT, keys));

        tap.key_down(VK_SHIFT_LEFT, DOWN, keys);
        tap.key_down(0x41, DOWN, keys); // 中间插了一个 A
        assert!(!tap.key_up(VK_SHIFT_LEFT, keys));
    }

    #[test]
    fn only_checked_keys_fire() {
        let tap = KeyTap::default();
        let keys = only(SwitchKey::Control);
        tap.key_down(VK_SHIFT_LEFT, DOWN, keys);
        assert!(!tap.key_up(VK_SHIFT_LEFT, keys));
        tap.key_down(VK_CONTROL_LEFT, DOWN, keys);
        assert!(tap.key_up(VK_CONTROL_LEFT, keys));

        // 一个都不勾、只勾组合键：修饰键单击都不算
        for keys in [SwitchKeys::NONE, only(SwitchKey::CtrlAltSpace)] {
            tap.key_down(VK_SHIFT_LEFT, DOWN, keys);
            assert!(!tap.key_up(VK_SHIFT_LEFT, keys));
            tap.key_down(VK_CONTROL_LEFT, DOWN, keys);
            assert!(!tap.key_up(VK_CONTROL_LEFT, keys));
        }
    }

    #[test]
    fn both_taps_work_when_both_are_checked_but_not_together() {
        let tap = KeyTap::default();
        let keys = only(SwitchKey::Shift).with(SwitchKey::Control, true);
        tap.key_down(VK_SHIFT_LEFT, DOWN, keys);
        assert!(tap.key_up(VK_SHIFT_LEFT, keys));
        tap.key_down(VK_CONTROL_LEFT, DOWN, keys);
        assert!(tap.key_up(VK_CONTROL_LEFT, keys));

        // Ctrl + Shift 一起按：谁抬起都不算
        tap.key_down(VK_CONTROL_LEFT, DOWN, keys);
        tap.key_down(VK_SHIFT_LEFT, DOWN, keys);
        assert!(!tap.key_up(VK_SHIFT_LEFT, keys));
        assert!(!tap.key_up(VK_CONTROL_LEFT, keys));
    }

    #[test]
    fn a_system_hotkey_cancels_the_pending_tap() {
        let tap = KeyTap::default();
        let keys = only(SwitchKey::Control);
        // Ctrl + Space 的 Space 被系统截走，这里只看到 Ctrl 按下又抬起
        tap.key_down(VK_CONTROL_LEFT, DOWN, keys);
        tap.cancel();
        assert!(!tap.key_up(VK_CONTROL_LEFT, keys));
    }

    #[test]
    fn auto_repeat_does_not_rearm() {
        let tap = KeyTap::default();
        let keys = only(SwitchKey::Shift);
        // 第 30 位为 1：自动重复，不算新按下
        let repeat = LPARAM(1 << 30);
        tap.key_down(VK_SHIFT_LEFT, DOWN, keys);
        assert!(tap.key_up(VK_SHIFT_LEFT, keys));
        tap.key_down(VK_SHIFT_LEFT, repeat, keys);
        assert!(!tap.key_up(VK_SHIFT_LEFT, keys));
    }
}
