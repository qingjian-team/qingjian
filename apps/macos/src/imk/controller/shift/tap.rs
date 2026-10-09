//! 单独轻按 Shift 的状态机；重叠修饰键和普通按键取消本次切换。

use std::time::{Duration, Instant};

/// 同一修饰键同一方向的转换在窗口内到达两次是 Chromium 系应用（Chrome / Edge）对 FlagsChanged
/// 的重复投递（间隔 1–2ms），第二遍要丢弃，否则会被当成按住第二个 Shift 把本次切换取消掉。
const DUPLICATE_WINDOW: Duration = Duration::from_millis(50);

#[derive(Default)]
pub(in crate::imk::controller) struct ShiftTap {
    side: Option<u16>,

    blocked: bool,

    /// 上一次到达的转换（键码、按下与否），连同时间戳；用于识别重复投递。
    last: Option<(u16, bool, Instant)>,
}

impl ShiftTap {
    pub(in crate::imk::controller) fn cancel(&mut self) {
        self.side = None;
        self.blocked = true;
    }

    /// 轻按进行中（已按下还没释放）才取消；全新状态不动，避免把下一次轻按标记成 blocked。
    pub(in crate::imk::controller) fn cancel_if_armed(&mut self) {
        if self.side.is_some() {
            self.cancel();
        }
    }

    pub(in crate::imk::controller) fn reset(&mut self) {
        self.side = None;
        self.blocked = false;
    }

    pub(in crate::imk::controller) fn changed(
        &mut self,
        key: u16,
        shift: bool,
        other: bool,
    ) -> bool {
        if let Some((last_key, last_shift, at)) = self.last
            && last_key == key
            && last_shift == shift
            && at.elapsed() < DUPLICATE_WINDOW
        {
            return false;
        }
        self.last = Some((key, shift, Instant::now()));
        if other || !matches!(key, 56 | 60) {
            self.cancel();
        }
        if !shift {
            let tapped = !other && !self.blocked && self.side == Some(key);
            self.reset();
            return tapped;
        }
        if !self.blocked {
            if self.side.is_some() {
                self.cancel();
            } else {
                self.side = Some(key);
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::ShiftTap;

    #[test]
    fn either_side_taps_once() {
        for side in [56, 60] {
            let mut tap = ShiftTap::default();
            assert!(!tap.changed(side, true, false));
            assert!(tap.changed(side, false, false));
            assert!(!tap.changed(side, false, false));
        }
    }

    #[test]
    fn combinations_and_overlap_cancel() {
        let mut tap = ShiftTap::default();
        tap.changed(56, true, false);
        tap.cancel();
        assert!(!tap.changed(56, false, false));
        tap.changed(56, true, false);
        tap.changed(60, true, false);
        assert!(!tap.changed(56, true, false));
        assert!(!tap.changed(60, false, false));
        tap.changed(56, true, false);
        tap.changed(55, true, true);
        assert!(!tap.changed(56, false, false));
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::{DUPLICATE_WINDOW, Instant, ShiftTap};

    #[test]
    fn missing_press_and_cancelled_focus_do_not_switch() {
        let mut tap = ShiftTap::default();
        assert!(!tap.changed(56, false, false));
        tap.changed(56, true, false);
        tap.cancel();
        assert!(!tap.changed(56, false, false));
        tap.changed(60, true, false);
        assert!(tap.changed(60, false, false));
    }

    #[test]
    fn disabled_caps_and_other_modifiers_block_until_release() {
        for key in [56, 60, 55, 59, 58, 57] {
            let mut tap = ShiftTap::default();
            tap.changed(key, true, true);
            assert!(!tap.changed(56, true, false));
            assert!(!tap.changed(56, false, false));
            tap.changed(56, true, false);
            assert!(tap.changed(56, false, false));
        }
    }

    #[test]
    fn chromium_duplicate_delivery_does_not_cancel_the_tap() {
        let mut tap = ShiftTap::default();
        // Edge / Chrome 每个转换送达两遍，间隔 1–2ms
        assert!(!tap.changed(56, true, false));
        assert!(!tap.changed(56, true, false));
        assert!(tap.changed(56, false, false));
        assert!(!tap.changed(56, false, false));
        // 重复窗口过后，正常的下一次轻按照常工作
        tap.last = Some((56, false, Instant::now() - DUPLICATE_WINDOW));
        assert!(!tap.changed(56, true, false));
        assert!(tap.changed(56, false, false));
    }

    #[test]
    fn duplicate_of_another_side_or_direction_is_processed() {
        let mut tap = ShiftTap::default();
        // 另一边 Shift 的按下不是重复
        tap.changed(56, true, false);
        assert!(!tap.changed(60, true, false));
        // 方向相反的转换不是重复：先按住再快速释放再按住，两次按下都能武装
        let mut tap = ShiftTap::default();
        tap.changed(56, true, false);
        assert!(tap.changed(56, false, false));
        assert!(!tap.changed(56, true, false));
        assert!(tap.changed(56, false, false));
    }
}
