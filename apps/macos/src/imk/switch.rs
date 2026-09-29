//! 中 / 英切换键的单击与组合键判定。
//!
//! 切换键来自 `[shortcut] switch_mode`，单击 Shift / 单击 Ctrl 可以都勾；Ctrl + Alt + Space 是组合键，按下即触发。
//! 按下切换键到抬起之间没插进别的键，就是一次单击。
//!
//! 事件来源分两路：组合键（空格键）与「别的键插进来作废」走 IMK 的 KeyDown / KeyUp（`key_down` / `key_up`）；
//! IMK 的 `handleEvent` 不收修饰键（Shift / Ctrl）事件，单击 Shift / Ctrl 由 CGEventTap 的 flagsChanged 差量判
//! （见 `modifier_tap.rs`），经 `modifier_change` 喂进同一个状态机。

use qingjian_platform::{Modifiers, SwitchKey, SwitchKeys};

/// 左 Shift。
const KEY_SHIFT_LEFT: u16 = 56;
/// 右 Shift。
const KEY_SHIFT_RIGHT: u16 = 60;
/// 左 Control。
const KEY_CONTROL_LEFT: u16 = 59;
/// 右 Control。
const KEY_CONTROL_RIGHT: u16 = 62;
/// 空格（组合键 Ctrl + Alt + Space 的主键）。
const KEY_SPACE: u16 = 49;

/// 切换键的单击 / 组合键状态机。
#[derive(Default)]
pub struct SwitchTap {
    /// 按下了哪个切换键、之后还没有别的键插进来；`None` 是没按或已被插键作废。
    pressed: Option<SwitchKey>,
}

impl SwitchTap {
    /// 一个键按下。是切换键就记下；是别的键就作废待定的单击。
    ///
    /// 返回 `true` 表示这次按下就是组合键 Ctrl + Alt + Space，应当立即切换模式。
    pub fn key_down(&mut self, key_code: u16, modifiers: Modifiers, keys: SwitchKeys) -> bool {
        // 组合键：Ctrl + Alt + Space（不带 ⌘，带 ⌘ 的归应用）
        if keys.ctrl_alt_space
            && key_code == KEY_SPACE
            && modifiers.control
            && modifiers.option
            && !modifiers.command
        {
            return true;
        }
        match tap_key(keys, key_code) {
            Some(key) => self.press(key),
            // 别的键插进来：待定的单击作废
            None => {
                self.pressed = None;
                false
            }
        }
    }

    /// 一个键抬起。切换键单独抬起返回 `true`（一次单击），一次抬起只算一次。
    pub fn key_up(&mut self, key_code: u16, keys: SwitchKeys) -> bool {
        match tap_key(keys, key_code) {
            Some(key) => self.release(key),
            None => false,
        }
    }

    /// 修饰键状态变化（来自 CGEventTap 的 flagsChanged，IMK 的 `handleEvent` 不收修饰键事件）：
    /// `key` 是变了的切换键，`pressed` 是按下还是抬起。只有勾着的切换键才算，返回 `true` 表示这次抬起构成单击。
    pub fn modifier_change(&mut self, key: SwitchKey, pressed: bool, keys: SwitchKeys) -> bool {
        if !keys.contains(key) {
            return false;
        }
        if pressed {
            self.press(key)
        } else {
            self.release(key)
        }
    }

    /// 一个切换键按下：记下待定的单击；两个切换键一起按（Ctrl + Shift 是系统换布局的键）不算。
    fn press(&mut self, key: SwitchKey) -> bool {
        self.pressed = match self.pressed {
            Some(other) if other != key => None,
            _ => Some(key),
        };
        false
    }

    /// 一个切换键抬起：之前记下的是它且没被插键作废，就是一次单击。
    fn release(&mut self, key: SwitchKey) -> bool {
        if self.pressed == Some(key) {
            self.pressed = None;
            true
        } else {
            false
        }
    }
}

/// 这个键码是不是勾着的单击切换键（左右两个都算）；组合键 Ctrl + Alt + Space 不走单击判定。
fn tap_key(keys: SwitchKeys, key_code: u16) -> Option<SwitchKey> {
    let is = |codes: [u16; 2]| codes.contains(&key_code);
    if keys.shift && is([KEY_SHIFT_LEFT, KEY_SHIFT_RIGHT]) {
        Some(SwitchKey::Shift)
    } else if keys.control && is([KEY_CONTROL_LEFT, KEY_CONTROL_RIGHT]) {
        Some(SwitchKey::Control)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn only(key: SwitchKey) -> SwitchKeys {
        SwitchKeys::NONE.with(key, true)
    }

    fn mods(option: bool, control: bool) -> Modifiers {
        Modifiers {
            option,
            shift: false,
            control,
            command: false,
        }
    }

    #[test]
    fn shift_tap_fires_only_when_nothing_else_interrupts() {
        let mut tap = SwitchTap::default();
        let keys = only(SwitchKey::Shift);
        assert!(!tap.key_down(KEY_SHIFT_LEFT, mods(false, false), keys));
        assert!(tap.key_up(KEY_SHIFT_LEFT, keys));
        // 一次抬起只算一次
        assert!(!tap.key_up(KEY_SHIFT_LEFT, keys));

        tap.key_down(KEY_SHIFT_LEFT, mods(false, false), keys);
        assert!(!tap.key_down(0x00, mods(false, false), keys)); // 中间插了一个 A
        assert!(!tap.key_up(KEY_SHIFT_LEFT, keys));
    }

    #[test]
    fn only_checked_keys_fire() {
        let mut tap = SwitchTap::default();
        let keys = only(SwitchKey::Control);
        tap.key_down(KEY_SHIFT_LEFT, mods(false, false), keys);
        assert!(!tap.key_up(KEY_SHIFT_LEFT, keys));
        tap.key_down(KEY_CONTROL_LEFT, mods(false, false), keys);
        assert!(tap.key_up(KEY_CONTROL_LEFT, keys));

        // 一个都不勾、只勾组合键：修饰键单击都不算
        for keys in [SwitchKeys::NONE, only(SwitchKey::CtrlAltSpace)] {
            let mut tap = SwitchTap::default();
            tap.key_down(KEY_SHIFT_LEFT, mods(false, false), keys);
            assert!(!tap.key_up(KEY_SHIFT_LEFT, keys));
            tap.key_down(KEY_CONTROL_LEFT, mods(false, false), keys);
            assert!(!tap.key_up(KEY_CONTROL_LEFT, keys));
        }
    }

    #[test]
    fn both_taps_work_when_both_are_checked_but_not_together() {
        let mut tap = SwitchTap::default();
        let keys = only(SwitchKey::Shift).with(SwitchKey::Control, true);
        tap.key_down(KEY_SHIFT_LEFT, mods(false, false), keys);
        assert!(tap.key_up(KEY_SHIFT_LEFT, keys));
        tap.key_down(KEY_CONTROL_LEFT, mods(false, false), keys);
        assert!(tap.key_up(KEY_CONTROL_LEFT, keys));

        // Ctrl + Shift 一起按：谁抬起都不算
        tap.key_down(KEY_CONTROL_LEFT, mods(false, false), keys);
        tap.key_down(KEY_SHIFT_LEFT, mods(false, false), keys);
        assert!(!tap.key_up(KEY_SHIFT_LEFT, keys));
        assert!(!tap.key_up(KEY_CONTROL_LEFT, keys));
    }

    #[test]
    fn right_handed_keys_count_too() {
        let mut tap = SwitchTap::default();
        let keys = only(SwitchKey::Shift);
        tap.key_down(KEY_SHIFT_RIGHT, mods(false, false), keys);
        assert!(tap.key_up(KEY_SHIFT_RIGHT, keys));
    }

    #[test]
    fn the_chord_fires_on_space_with_ctrl_and_alt() {
        let mut tap = SwitchTap::default();
        let keys = only(SwitchKey::CtrlAltSpace);
        // 只带 Ctrl 不算
        assert!(!tap.key_down(KEY_SPACE, mods(false, true), keys));
        // Ctrl + Alt 按下即触发
        assert!(tap.key_down(KEY_SPACE, mods(true, true), keys));
        // 带 ⌘ 的归应用
        let with_command = Modifiers {
            option: true,
            shift: false,
            control: true,
            command: true,
        };
        assert!(!tap.key_down(KEY_SPACE, with_command, keys));
    }

    #[test]
    fn the_chord_does_not_arm_a_pending_tap() {
        let mut tap = SwitchTap::default();
        let keys = only(SwitchKey::CtrlAltSpace);
        // 组合键触发后，空格抬起不算单击
        assert!(tap.key_down(KEY_SPACE, mods(true, true), keys));
        assert!(!tap.key_up(KEY_SPACE, keys));
    }

    #[test]
    fn modifier_change_fires_a_tap_on_release() {
        let mut tap = SwitchTap::default();
        let keys = only(SwitchKey::Shift);
        // 按下不触发，抬起才触发一次
        assert!(!tap.modifier_change(SwitchKey::Shift, true, keys));
        assert!(tap.modifier_change(SwitchKey::Shift, false, keys));
        // 一次抬起只算一次
        assert!(!tap.modifier_change(SwitchKey::Shift, false, keys));
    }

    #[test]
    fn modifier_tap_is_invalidated_by_an_interrupting_key() {
        let mut tap = SwitchTap::default();
        let keys = only(SwitchKey::Shift);
        // Shift 按下后插了一个普通键（走 IMK 的 key_down），抬起不算单击
        tap.modifier_change(SwitchKey::Shift, true, keys);
        tap.key_down(0x00, mods(false, false), keys);
        assert!(!tap.modifier_change(SwitchKey::Shift, false, keys));
    }

    #[test]
    fn modifier_change_only_fires_for_checked_keys() {
        let mut tap = SwitchTap::default();
        // 只勾 Ctrl：Shift 的按下抬起都不算
        let keys = only(SwitchKey::Control);
        tap.modifier_change(SwitchKey::Shift, true, keys);
        assert!(!tap.modifier_change(SwitchKey::Shift, false, keys));
        tap.modifier_change(SwitchKey::Control, true, keys);
        assert!(tap.modifier_change(SwitchKey::Control, false, keys));
    }

    #[test]
    fn two_modifiers_held_together_do_not_fire() {
        let mut tap = SwitchTap::default();
        let keys = only(SwitchKey::Shift).with(SwitchKey::Control, true);
        // 两个切换键一起按：谁抬起都不算
        tap.modifier_change(SwitchKey::Control, true, keys);
        tap.modifier_change(SwitchKey::Shift, true, keys);
        assert!(!tap.modifier_change(SwitchKey::Shift, false, keys));
        assert!(!tap.modifier_change(SwitchKey::Control, false, keys));
    }
}
