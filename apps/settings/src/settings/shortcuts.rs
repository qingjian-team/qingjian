//! 快捷键页的「恢复默认」。

use qingjian_platform::{PAGE_KEY_OPTIONS, ShortcutConfig};

use super::Settings;
use crate::support::Feature;

impl Settings {
    /// 快捷键页的各项（含翻页键、模式键）写回缺省；辅码触发键在「输入」页，不动。
    pub(super) fn reset_shortcuts(&mut self) {
        let defaults = ShortcutConfig::default();
        self.write("general", "page_keys", PAGE_KEY_OPTIONS[0]);
        self.write(
            "shortcut",
            "expression",
            defaults.mode.expression.to_string(),
        );
        self.write("shortcut", "question", defaults.mode.question.to_string());
        self.write("shortcut", "question_mark", defaults.mode.question_mark);
        self.write("shortcut", "translation", defaults.translation.key());
        self.write(
            "shortcut",
            "translation_second",
            defaults.translation_second.key(),
        );
        self.write(
            "shortcut",
            "delete_candidate",
            defaults.delete_candidate.key(),
        );
        self.write(
            "shortcut",
            "translate_selection",
            defaults.translate_selection.key_string(),
        );
        if Feature::CloudRequestControls.here() {
            self.write("shortcut", "predict", defaults.predict.key_string());
        }
        if Feature::HighlightKeys.here() {
            self.write("shortcut", "highlight_keys", defaults.highlight_keys.key());
        }
        if Feature::ExtraPageKeys.here() {
            self.write_list("shortcut", "extra_page_keys", &[]);
        }
        if Feature::TogglePunctuation.here() {
            self.write("shortcut", "toggle_punctuation", "");
        }
        self.recording = None;
    }
}
