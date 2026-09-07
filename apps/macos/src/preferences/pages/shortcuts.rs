//! 「快捷键」页：翻页键、模式键、译词上屏 / 删候选 / 翻译选中文字的组合键。

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::NSPopUpButton;
use qingjian_core::ModeKeys;
use qingjian_platform::{Config, PAGE_KEY_OPTIONS};

use crate::preferences::controls::{
    GROUP_GAP, button, note, note_full, page_keys_label, row_popup, row_recorder, select,
};
use crate::preferences::key_recorder::KeyRecorder;
use crate::preferences::layout::{Layout, PAGE_PADDING, ROW_HEIGHT};
use crate::preferences::setting::Setting;
use crate::preferences::target::PreferencesTarget;

pub struct ShortcutsPage {
    /// 翻页键对。
    page_keys: Retained<NSPopUpButton>,

    /// 表达式模式键。
    expression: Retained<NSPopUpButton>,

    /// 问字模式键。
    question: Retained<NSPopUpButton>,

    /// 上屏第一个译词的修饰键。
    translation: Retained<KeyRecorder>,

    /// 上屏第二个译词的修饰键。
    translation_second: Retained<KeyRecorder>,

    /// 删除候选的修饰键。
    delete_candidate: Retained<KeyRecorder>,

    /// 翻译选中文字的组合键。
    translate_selection: Retained<KeyRecorder>,
}

impl ShortcutsPage {
    pub fn build(layout: &mut Layout, mtm: MainThreadMarker, target: &PreferencesTarget) -> Self {
        let page_key_titles: Vec<String> = PAGE_KEY_OPTIONS
            .iter()
            .map(|k| page_keys_label(k))
            .collect();
        let page_keys = row_popup(
            layout,
            mtm,
            "翻页键",
            &page_key_titles,
            Setting::PageKeys,
            target,
        );
        note(
            layout,
            mtm,
            "选「，  。」时组句中敲逗号句号是翻页，不再是上屏加标点。",
        );
        let key_titles: Vec<String> = ModeKeys::CANDIDATES.iter().map(char::to_string).collect();
        let expression = row_popup(
            layout,
            mtm,
            "表达式模式键",
            &key_titles,
            Setting::ExpressionKey,
            target,
        );
        let question = row_popup(
            layout,
            mtm,
            "问字模式键",
            &key_titles,
            Setting::QuestionKey,
            target,
        );
        note(
            layout,
            mtm,
            "这两个字母开头进模式：v1+2 出 3，usangemu 问「三个木」（需要云服务），u4e00 出对应的字符；? 开头永远是问字。两个键不能相同。",
        );
        layout.space(GROUP_GAP);
        let translation = row_recorder(
            layout,
            mtm,
            "上屏第一个译词",
            Setting::TranslationKeys,
            true,
            target,
        );
        let translation_second = row_recorder(
            layout,
            mtm,
            "上屏第二个译词",
            Setting::TranslationSecondKeys,
            true,
            target,
        );
        note(
            layout,
            mtm,
            "按住修饰键再按候选序号，上屏的是候选右侧的译词而不是中文；候选有两个译词时第二组键上屏后一个。两组不能相同。",
        );
        layout.space(GROUP_GAP);
        let delete_candidate = row_recorder(
            layout,
            mtm,
            "删除候选",
            Setting::DeleteCandidateKeys,
            true,
            target,
        );
        note(
            layout,
            mtm,
            "按住修饰键再按候选序号：自己造的词、云端选过的词整个删掉；词库里的词清掉对它的学习记录，回到原来的排序。组句中要打感叹号先把词上屏。",
        );
        layout.space(GROUP_GAP);
        let translate_selection = row_recorder(
            layout,
            mtm,
            "翻译选中的文字",
            Setting::TranslateSelectionKeys,
            false,
            target,
        );
        note(
            layout,
            mtm,
            "在应用里选中一段文字再按这个键，译文（学习语言）出现在候选窗口：回车替换选中的文字，Esc 保留原文。需要开着云服务。",
        );
        layout.space(GROUP_GAP);
        note_full(
            layout,
            mtm,
            "改快捷键：点一下右边的按钮，再按下新的组合键（要带修饰键 ⌃ ⌥ ⇧ ⌘），Esc 取消。避开 ⌃+数字（切换桌面）和 ⌘+数字 / ⌘T（应用常用键）。",
        );
        let reset = button(mtm, "恢复默认快捷键", Setting::ResetShortcuts, target);
        layout.place(&reset, PAGE_PADDING, 160.0, ROW_HEIGHT + 4.0);
        layout.next_row(ROW_HEIGHT + 4.0);
        Self {
            page_keys,
            expression,
            question,
            translation,
            translation_second,
            delete_candidate,
            translate_selection,
        }
    }

    pub fn sync(&self, config: &Config) {
        let (previous, next) = config.general.page_keys();
        let pair = format!("{previous}{next}");
        select(
            &self.page_keys,
            PAGE_KEY_OPTIONS.iter().position(|k| *k == pair),
        );
        let keys = config.shortcut.mode.sanitized();
        select(
            &self.expression,
            ModeKeys::CANDIDATES
                .iter()
                .position(|k| *k == keys.expression),
        );
        select(
            &self.question,
            ModeKeys::CANDIDATES
                .iter()
                .position(|k| *k == keys.question),
        );
        let (first, second) = config.shortcut.translation_keys();
        self.translation.show(&first.key(), &first.label());
        self.translation_second.show(&second.key(), &second.label());
        let delete = config.shortcut.delete_keys();
        self.delete_candidate.show(&delete.key(), &delete.label());
        let translate = config.shortcut.translate_selection;
        self.translate_selection
            .show(&translate.key_string(), &translate.label());
    }
}
