//! Windows 设置卡片行首的单色线条图标：哪一页、标题是哪条文案的行用哪个图标（`assets/rows/` 里的文件名）。
//! 图标从设计原型的线性图标导出；页里同名的行（「启用」）靠页区分。

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use bamboo_slip::{Icon, SvgImage};

use crate::i18n::tr;
use crate::page_id::PageId;

/// （页，行标题的文案键，图标名）。文案带参数（`{key}`）的按参数前后的字比。
const ROWS: [(PageId, &str, &str); 80] = [
    (PageId::Input, "input.pinyin", "keyboard"),
    (PageId::Input, "input.hold_selections", "pages"),
    (PageId::Input, "input.code", "text"),
    (PageId::Input, "input.key_hint", "grid"),
    (PageId::Input, "input.auto_phrase", "sparkle"),
    (PageId::Input, "input.spelling_correction", "spell"),
    (PageId::Input, "input.aux_enabled", "filter"),
    (PageId::Candidates, "candidates.page_size", "list"),
    (PageId::Candidates, "candidates.numpad_digit", "calc"),
    (PageId::Candidates, "candidates.translation", "globe"),
    (PageId::Candidates, "candidates.traditional", "text"),
    (PageId::Candidates, "candidates.translation_pos", "abc"),
    (
        PageId::Candidates,
        "candidates.english_pronunciation",
        "speaker",
    ),
    (PageId::Candidates, "candidates.show_tones", "wave"),
    (PageId::Candidates, "candidates.emoji", "smile"),
    (PageId::Candidates, "candidates.kaomoji", "phrases"),
    (PageId::Candidates, "candidates.symbols", "omega"),
    (PageId::Candidates, "candidates.learning", "sparkle"),
    (PageId::Candidates, "candidates.local_model", "flask"),
    (PageId::Candidates, "candidates.english_in_chinese", "abc"),
    (PageId::Language, "language.english_mode", "lang"),
    (PageId::Language, "language.double_shift_input", "keyboard"),
    (PageId::Language, "language.shift_letter", "shift"),
    (PageId::Language, "language.chinese_punctuation", "quote"),
    (PageId::Language, "language.english_punctuation", "abc"),
    (PageId::Language, "language.half_width_symbols", "text"),
    (PageId::Language, "language.punct_commits", "comma"),
    (PageId::Language, "language.pair_punctuation", "brackets"),
    (PageId::Language, "language.auto_space", "space"),
    (PageId::Language, "language.status_bar", "pill"),
    (PageId::Language, "language.menubar_indicator", "menubar"),
    (PageId::Appearance, "appearance.appearance", "sun"),
    (PageId::Appearance, "appearance.layout", "layout"),
    (PageId::Appearance, "appearance.preedit", "edit"),
    (PageId::Appearance, "appearance.renderer", "window"),
    (PageId::Appearance, "appearance.font", "text"),
    (PageId::Appearance, "appearance.candidate_size", "textsize"),
    (PageId::Appearance, "appearance.candidate_weight", "bold"),
    (PageId::Appearance, "appearance.annotation_size", "abc"),
    (PageId::Appearance, "appearance.animations", "motion"),
    (PageId::Shortcuts, "shortcuts.page_keys", "pages"),
    (PageId::Shortcuts, "shortcuts.expression", "calc"),
    (PageId::Shortcuts, "shortcuts.lookup", "search"),
    (PageId::Shortcuts, "shortcuts.question_lookup", "info"),
    (PageId::Shortcuts, "shortcuts.extra_page_keys", "pages"),
    (PageId::Shortcuts, "shortcuts.highlight_keys", "sort"),
    (
        PageId::Shortcuts,
        "shortcuts.extra_candidate_navigation",
        "keyboard",
    ),
    (PageId::Shortcuts, "shortcuts.traditional", "text"),
    (PageId::Shortcuts, "shortcuts.enter_translation", "keyboard"),
    (PageId::Shortcuts, "shortcut.commit_candidate", "space"),
    (PageId::Shortcuts, "shortcut.commit_raw", "abc"),
    (PageId::Shortcuts, "shortcut.translation", "globe"),
    (PageId::Shortcuts, "shortcut.translation_second", "globe"),
    (PageId::Shortcuts, "shortcut.translate_selection", "lang"),
    (PageId::Shortcuts, "shortcut.predict", "cloud"),
    (PageId::Shortcuts, "shortcut.translate_panel", "omega"),
    (PageId::Shortcuts, "shortcuts.single_chars", "text"),
    (PageId::Shortcuts, "shortcuts.next_word", "right"),
    (PageId::Phrases, "phrases.new_phrase", "plus"),
    (PageId::Phrases, "phrases.transfer", "download"),
    (PageId::Phrases, "phrases.system_replacements", "text"),
    (PageId::Cloud, "cloud.enabled", "cloud"),
    (PageId::Cloud, "cloud.base_url", "link"),
    (PageId::Cloud, "cloud.model", "sparkle"),
    (PageId::Cloud, "cloud.api_key", "lock"),
    (PageId::Cloud, "cloud.connection", "refresh"),
    (PageId::Advanced, "advanced.config_file", "doc"),
    (PageId::Advanced, "advanced.transfer", "download"),
    (PageId::Advanced, "advanced.verbose_log", "log"),
    (PageId::Advanced, "advanced.log_files", "folder"),
    (PageId::Advanced, "advanced.input_log", "clock"),
    (PageId::Advanced, "advanced.clear_input_log", "trash"),
    (
        PageId::Experimental,
        "experimental.clipboard_enabled",
        "pages",
    ),
    (PageId::Experimental, "experimental.clipboard_sync", "sync"),
    (
        PageId::Experimental,
        "experimental.clipboard_history",
        "trash",
    ),
    (PageId::About, "about.auto_check", "refresh"),
    (PageId::About, "about.channel", "flask"),
    (PageId::About, "about.check_update", "download"),
    (PageId::Shortcuts, "shortcuts.punctuation_width", "comma"),
    (PageId::About, "about.new_version", "sparkle"),
];

/// 随包的图标文件。
macro_rules! sources {
    ($($name:literal),* $(,)?) => {
        [$(($name, include_bytes!(concat!("../../../assets/rows/", $name, ".svg")) as &[u8])),*]
    };
}

const SOURCES: [(&str, &[u8]); 48] = sources!(
    "abc", "bold", "brackets", "calc", "clock", "cloud", "comma", "doc", "download", "edit",
    "filter", "flask", "folder", "globe", "grid", "info", "keyboard", "lang", "layout", "link",
    "list", "lock", "log", "menubar", "motion", "omega", "pages", "phrases", "pill", "plus",
    "quote", "refresh", "right", "search", "shift", "smile", "sort", "space", "sparkle", "speaker",
    "spell", "sun", "sync", "text", "textsize", "trash", "wave", "window",
);

/// 第一次用时解析全部图标。
fn icons() -> &'static HashMap<&'static str, Arc<SvgImage>> {
    static ICONS: OnceLock<HashMap<&'static str, Arc<SvgImage>>> = OnceLock::new();
    ICONS.get_or_init(|| {
        SOURCES
            .iter()
            .map(|(name, data)| {
                let svg = SvgImage::parse(data).expect("bundled row icon is valid SVG");
                (*name, Arc::new(svg))
            })
            .collect()
    })
}

/// `page` 页标题为 `title` 的行用的图标。
pub fn row_icon(page: PageId, title: &str) -> Option<Arc<dyn Icon>> {
    let (_, _, name) = ROWS
        .iter()
        .find(|(row_page, key, _)| *row_page == page && matches(tr(key), title))?;
    let icon: Arc<dyn Icon> = icons().get(name)?.clone();
    Some(icon)
}

/// `title` 是不是文案 `text` 填好参数后的样子（只认一个参数）。
fn matches(text: &str, title: &str) -> bool {
    match (text.find('{'), text.find('}')) {
        (Some(open), Some(close)) if open < close => {
            let (prefix, suffix) = (&text[..open], &text[close + 1..]);
            title.len() > prefix.len() + suffix.len()
                && title.starts_with(prefix)
                && title.ends_with(suffix)
        }
        _ => text == title,
    }
}

#[cfg(test)]
mod tests {
    use super::{ROWS, SOURCES, icons, matches};
    use crate::i18n::tr;

    #[test]
    fn every_row_names_a_bundled_icon_and_a_real_text() {
        let icons = icons();
        assert_eq!(icons.len(), SOURCES.len());
        for (_, key, name) in ROWS {
            assert!(icons.contains_key(name), "{name}");
            assert_ne!(tr(key), key, "{key}");
        }
    }

    #[test]
    fn texts_with_a_parameter_match_the_filled_title() {
        assert!(matches("{key} 切换全半角标点", "Ctrl + . 切换全半角标点"));
        assert!(matches("新版本 {version}", "新版本 0.3.6"));
        assert!(!matches("新版本 {version}", "新版本 "));
        assert!(matches("拼音", "拼音"));
        assert!(!matches("拼音", "拼音行"));
    }
}
