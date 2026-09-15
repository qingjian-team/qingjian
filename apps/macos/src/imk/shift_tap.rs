//! 单击 Shift 手势：在松开时识别，排除组合键、双 Shift、长按和跨会话残留。

use objc2_app_kit::NSEventModifierFlags;

/// 单击允许的最长按住时间（秒），避免长按选择或等待时意外切换。
const TAP_DURATION: f64 = 0.5;

#[derive(Debug, Default)]
pub struct ShiftTap {
    /// 上一事件中是否有任一 Shift 按住，避免把双 Shift 的中途松开当成新按下。
    down: bool,

    /// 待确认的物理 Shift 键与按下时间；组合键、鼠标操作都会取消。
    pending: Option<(u16, f64)>,
}

impl ShiftTap {
    /// 处理 flagsChanged；仅同一 Shift 在时限内独立按下再松开时返回 true。
    /// Caps Lock 的锁定状态不影响识别；实际按下 Caps Lock 或其他修饰键会取消手势。
    pub fn flags_changed(&mut self, key: u16, flags: NSEventModifierFlags, time: f64) -> bool {
        let shift = flags.contains(NSEventModifierFlags::Shift);
        let other = flags.intersects(
            NSEventModifierFlags::Command
                | NSEventModifierFlags::Control
                | NSEventModifierFlags::Option
                | NSEventModifierFlags::Function,
        );
        // 56 / 60 是 macOS 左 / 右 Shift 的事件键码，只用于识别物理事件。
        let is_shift = matches!(key, 56 | 60);
        let pending = self.pending.take();
        let tapped = !shift
            && self.down
            && is_shift
            && !other
            && pending.is_some_and(|(pressed, start)| {
                pressed == key && (0.0..=TAP_DURATION).contains(&(time - start))
            });
        if shift && !self.down && is_shift && !other {
            self.pending = Some((key, time));
        }
        self.down = shift;
        tapped
    }

    /// 其他按键或鼠标动作发生，取消单击；保留按住状态直到收到松开事件。
    pub fn cancel(&mut self) {
        self.pending = None;
    }

    /// 失焦、输入源切换或重新开始录制时清空，孤立的松开事件不会触发动作。
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn either_shift_toggles_only_on_release_and_once() {
        for key in [56, 60] {
            let mut tap = ShiftTap::default();
            assert!(!tap.flags_changed(key, NSEventModifierFlags::Shift, 1.0));
            assert!(tap.flags_changed(key, NSEventModifierFlags::empty(), 1.2));
            assert!(!tap.flags_changed(key, NSEventModifierFlags::empty(), 1.3));
        }
    }

    #[test]
    fn typing_or_mouse_selection_cancels_the_tap() {
        let mut tap = ShiftTap::default();
        tap.flags_changed(56, NSEventModifierFlags::Shift, 1.0);
        tap.cancel();
        assert!(!tap.flags_changed(56, NSEventModifierFlags::empty(), 1.2));
    }

    #[test]
    fn modifier_chords_and_overlapping_shifts_never_toggle() {
        let mut tap = ShiftTap::default();
        tap.flags_changed(56, NSEventModifierFlags::Shift, 1.0);
        tap.flags_changed(
            59,
            NSEventModifierFlags::Shift | NSEventModifierFlags::Control,
            1.1,
        );
        tap.flags_changed(59, NSEventModifierFlags::Shift, 1.2);
        assert!(!tap.flags_changed(56, NSEventModifierFlags::empty(), 1.3));
        tap.flags_changed(56, NSEventModifierFlags::Shift, 2.0);
        tap.flags_changed(60, NSEventModifierFlags::Shift, 2.1);
        tap.flags_changed(56, NSEventModifierFlags::Shift, 2.2);
        assert!(!tap.flags_changed(60, NSEventModifierFlags::empty(), 2.3));
    }

    #[test]
    fn long_hold_and_focus_changes_do_not_toggle() {
        let mut tap = ShiftTap::default();
        tap.flags_changed(56, NSEventModifierFlags::Shift, 1.0);
        assert!(!tap.flags_changed(56, NSEventModifierFlags::empty(), 2.0));
        tap.flags_changed(56, NSEventModifierFlags::Shift, 3.0);
        tap.reset();
        assert!(!tap.flags_changed(56, NSEventModifierFlags::empty(), 3.1));
        assert!(!tap.flags_changed(56, NSEventModifierFlags::empty(), 4.0));
    }

    #[test]
    fn caps_lock_state_is_allowed_but_pressing_it_cancels() {
        let mut tap = ShiftTap::default();
        tap.flags_changed(
            60,
            NSEventModifierFlags::Shift | NSEventModifierFlags::CapsLock,
            1.0,
        );
        assert!(tap.flags_changed(60, NSEventModifierFlags::CapsLock, 1.2));
        tap.flags_changed(60, NSEventModifierFlags::Shift, 2.0);
        tap.flags_changed(
            57,
            NSEventModifierFlags::Shift | NSEventModifierFlags::CapsLock,
            2.1,
        );
        assert!(!tap.flags_changed(60, NSEventModifierFlags::CapsLock, 2.2));
    }
}
