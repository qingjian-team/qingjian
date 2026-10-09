//! 中英状态与候选策略，硬件观察与带客户端的上屏分开。

mod state;

use crate::host::Host;
use crate::imk::modifiers;
pub(crate) use state::InputState;

impl Host {
    pub fn effective_english(&self) -> bool {
        self.input.english
    }

    pub fn update_input_indicator(&mut self) {
        self.indicator.update(self.input.english);
    }

    pub fn poll_input_mode(&mut self) {
        self.input.observe_caps(modifiers::caps_lock_on());
        self.update_input_indicator();
    }
}
