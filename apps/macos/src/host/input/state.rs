//! 逻辑中英与候选策略；Caps 灯只用于边沿去重，不覆盖 Shift 的选择。

pub(crate) struct InputState {
    pub english: bool,

    pub candidates: bool,

    observed_caps: bool,

    pub pending: bool,
}

impl InputState {
    pub fn new(caps: bool) -> Self {
        Self {
            english: caps,
            candidates: caps,
            observed_caps: caps,
            pending: false,
        }
    }

    /// 回到青简时同步灯的基线，不把外部灯变化当作切换动作。
    pub fn reset_chinese(&mut self, caps: bool) {
        self.english = false;
        self.candidates = false;
        self.observed_caps = caps;
        self.pending = true;
    }

    pub fn desired_caps(&self) -> bool {
        self.english && self.candidates
    }

    /// 成功回读后直接同步基线，自发灯变化不解释为用户切换。
    pub fn confirm_caps(&mut self, caps: bool) {
        self.observed_caps = caps;
    }

    pub fn observe_caps(&mut self, caps: bool) {
        if self.observed_caps == caps {
            return;
        }
        self.observed_caps = caps;
        let candidates = !(self.english && self.candidates);
        self.english = candidates;
        self.candidates = candidates;
        self.pending = true;
    }

    pub fn shift(&mut self) {
        let passthrough = self.english && !self.candidates;
        self.english = !passthrough;
        self.candidates = false;
        self.pending = true;
    }

    pub fn disable_shift(&mut self) {
        if self.english && !self.candidates {
            self.english = false;
            self.pending = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::InputState;

    #[test]
    fn all_six_transitions() {
        for (english, candidates) in [(false, false), (true, true), (true, false)] {
            let mut state = InputState::new(false);
            state.english = english;
            state.candidates = candidates;
            state.shift();
            assert_eq!(
                (state.english, state.candidates),
                (!english || candidates, false)
            );
            state.english = english;
            state.candidates = candidates;
            state.observe_caps(true);
            let target = !(english && candidates);
            assert_eq!((state.english, state.candidates), (target, target));
        }
    }

    #[test]
    fn lit_caps_does_not_override_shift_and_pending_survives_roundtrip() {
        let mut state = InputState::new(true);
        state.shift();
        assert_eq!((state.english, state.candidates), (true, false));
        state.observe_caps(true);
        assert!(!state.candidates);
        state.shift();
        state.observe_caps(true);
        assert!(!state.english);
        state.observe_caps(false);
        assert!(state.candidates);
        state.observe_caps(true);
        assert!(!state.english);
        assert!(state.pending);
    }

    #[test]
    fn disabling_shift_only_exits_passthrough() {
        let mut state = InputState::new(true);
        state.disable_shift();
        assert!(state.candidates);
        state.shift();
        state.disable_shift();
        assert!(!state.english);
        assert!(state.pending);
    }
}

#[cfg(test)]
mod reentry_tests {
    use super::InputState;

    #[test]
    fn reentry_resets_both_english_policies_and_caps_baseline() {
        for caps in [false, true] {
            for candidates in [false, true] {
                let mut state = InputState::new(!caps);
                state.english = true;
                state.candidates = candidates;
                state.reset_chinese(caps);
                state.observe_caps(caps);
                assert!(!state.english);
                assert!(!state.candidates);
                assert!(state.pending);
                state.pending = false;
                state.observe_caps(!caps);
                assert!(state.english);
                assert!(state.candidates);
            }
        }
    }
}

#[cfg(test)]
mod caps_sync_tests {
    use super::InputState;

    #[test]
    fn automatic_off_feedback_preserves_passthrough() {
        let mut state = InputState::new(true);
        state.shift();
        assert!(!state.desired_caps());
        state.confirm_caps(false);
        state.observe_caps(false);
        assert!(state.english);
        assert!(!state.candidates);
        state.observe_caps(true);
        assert!(state.desired_caps());
        state.reset_chinese(true);
        state.confirm_caps(false);
        state.observe_caps(false);
        assert!(!state.english);
    }
}
