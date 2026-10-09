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
