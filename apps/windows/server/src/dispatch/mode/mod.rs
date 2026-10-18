//! 中英模式的归属：缺省全局共享，可选按应用记忆；未知宿主退回会话隔离。

#[cfg(test)]
mod tests;

use qingjian_platform::protocol::SessionId;

use super::{Router, RouterConfig};

impl Router {
    pub(super) fn activate_mode(&mut self, session: SessionId) {
        self.mode_session = Some(session);
        let previous = self.english;
        if !self.config.english_mode {
            self.english = false;
        } else if self.config.remember_mode_per_app {
            let initial = self.config.default_english;
            self.english = match self.sessions.get_mut(&session) {
                Some(info) => match info.app.as_deref().filter(|app| !app.is_empty()) {
                    Some(app) => *self.app_modes.entry(app.to_lowercase()).or_insert(initial),
                    None => *info.english.get_or_insert(initial),
                },
                None => initial,
            };
        }
        if previous != self.english {
            self.reconcile_status();
        }
    }

    pub(super) fn remember_mode(&mut self) {
        if !self.config.remember_mode_per_app {
            return;
        }
        let Some(info) = self.mode_session.and_then(|id| self.sessions.get_mut(&id)) else {
            return;
        };
        match info.app.as_deref().filter(|app| !app.is_empty()) {
            Some(app) => {
                self.app_modes.insert(app.to_lowercase(), self.english);
            }
            None => info.english = Some(self.english),
        }
    }

    /// 只有模式策略变更才重置；字体、标点等无关配置热加载不能覆盖用户刚切出的模式。
    pub(super) fn reload_mode(&mut self, next: &RouterConfig) {
        if self.config.english_mode == next.english_mode
            && self.config.default_english == next.default_english
            && self.config.remember_mode_per_app == next.remember_mode_per_app
        {
            return;
        }
        self.english = next.english_mode && next.default_english;
        self.app_modes.clear();
        for info in self.sessions.values_mut() {
            info.english = None;
        }
    }
}
