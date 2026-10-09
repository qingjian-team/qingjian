//! 单独轻按 Shift 的状态机；重叠修饰键和普通按键取消本次切换。

#[derive(Default)]
pub(in crate::imk::controller) struct ShiftTap {
    side: Option<u16>,

    blocked: bool,
}

impl ShiftTap {
    pub(in crate::imk::controller) fn cancel(&mut self) {
        self.side = None;
        self.blocked = true;
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
    use super::ShiftTap;

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
}
