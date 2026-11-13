//! 复用共用方案加载；实际变化时静默取消各上下文的未提交输入。

use std::path::Path;

use qingjian_platform::{Config, PreparedScheme};

use super::Router;

impl Router {
    pub(super) fn apply_schemes(&mut self, config: &Config, root: &Path, user_dir: &Path) {
        let prepared = self.scheme_slot.prepare(
            config,
            Some(user_dir),
            Some(&root.join("data/generated/schemes")),
            self.engine.code_scheme().is_some(),
        );
        self.apply_prepared_schemes(prepared);
    }

    pub(super) fn apply_prepared_schemes(&mut self, prepared: PreparedScheme) {
        let previous = self.scheme_slot.clone();
        let pinyin = prepared.pinyin();
        self.scheme_slot.apply_prepared(&mut self.engine, prepared);
        if previous == self.scheme_slot && self.scheme == pinyin {
            return;
        }
        self.scheme = pinyin;
        self.end_translation();
        self.stop_rescoring();
        self.engine.discard_input();
        self.composed = None;
        self.sentence = None;
        self.notice = None;
        self.highlight = 0;
        self.navigated = false;
        self.trace = None;
        for info in self.sessions.values_mut() {
            info.engine.discard_input();
            info.composed = None;
            info.highlight = 0;
            info.navigated = false;
            info.switch_tap = None;
            info.display_frame = None;
            info.last_frame = None;
            if let Some(identity) = &mut info.display_identity {
                self.display_revision += 1;
                identity.revision = self.display_revision;
            }
        }
    }
}
