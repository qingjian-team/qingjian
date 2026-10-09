//! 输入源边界：真实离开后再进入才重置，重复激活和同源通知不重置。

#[derive(Default)]
pub(crate) struct SourceState {
    selected: Option<bool>,
}

impl SourceState {
    pub fn observe(&mut self, selected: bool) -> bool {
        let entered = selected && self.selected != Some(true);
        self.selected = Some(selected);
        entered
    }
}

#[cfg(test)]
mod tests {
    use super::SourceState;

    #[test]
    fn only_entry_from_another_source_resets() {
        let mut state = SourceState::default();
        assert!(state.observe(true));
        assert!(!state.observe(true));
        assert!(!state.observe(false));
        assert!(!state.observe(false));
        assert!(state.observe(true));
        assert!(!state.observe(true));
    }
}
