//! 复用共用方案加载；实际变化时静默取消各上下文的未提交输入。

use std::path::Path;

use qingjian_platform::Config;

use super::Router;

impl Router {
    pub(super) fn apply_schemes(&mut self, config: &Config, root: &Path, user_dir: &Path) {
        let previous = self.scheme_slot.clone();
        let pinyin = config.general.scheme();
        self.scheme_slot.apply(
            &mut self.engine,
            config,
            Some(user_dir),
            Some(&root.join("assets/schemes")),
        );
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
