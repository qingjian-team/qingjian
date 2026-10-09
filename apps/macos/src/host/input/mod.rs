//! 中英状态与候选策略，硬件观察与带客户端的上屏分开。

mod monitor;
mod source;
mod state;

pub(crate) use monitor::SourceMonitor;
pub(crate) use source::SourceState;

use crate::host::Host;
use crate::imk::modifiers;
pub(crate) use state::InputState;

impl Host {
    pub fn sync_input_source(&mut self) {
        let Some(selected) = crate::app::input_source::current_is_qingjian() else {
            return;
        };
        if self.input_source.observe(selected) {
            self.input.reset_chinese(modifiers::caps_lock_on());
            self.input_config_generation = self.input_config_generation.wrapping_add(1);
            self.update_input_indicator();
        }
    }

    pub fn sync_caps_lock(&mut self) {
        let desired = self.input.desired_caps();
        match modifiers::set_caps_lock(desired) {
            Ok(()) => self.input.confirm_caps(desired),
            Err(error) => tracing::warn!(%error, "模式已切换，但 Caps 灯未同步"),
        }
    }

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
