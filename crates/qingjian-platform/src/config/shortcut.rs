use qingjian_core::ModeKeys;
use serde::{Deserialize, Serialize};

use super::PageKeys;
use super::commit_key::{CommitKey, CommitKeys};
use super::highlight_keys::HighlightKeys;
use super::key_combo::KeyCombo;
use super::modifiers::Modifiers;
use super::switch_key::SwitchKeys;

/// 配置文件 `[shortcut]` 分节：前缀模式键（Core 的 [`ModeKeys`]）加壳层的修饰键组合。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShortcutConfig {
    /// 表达式 / 问字模式键，键名与以前一样直接在分节下（`expression` / `question`）。
    #[serde(flatten)]
    pub mode: ModeKeys,

    /// 中 / 英切换键（Windows、Linux 用），可多选：`["shift", "control", "ctrl+alt+space"]`。详见 [`SwitchKeys`]。
    pub switch_mode: SwitchKeys,

    /// 候选高亮由 `↑` / `↓` 还是 `←` / `→` 移动（Windows 用）。详见 [`HighlightKeys`]。
    pub highlight_keys: HighlightKeys,

    /// 候选中额外启用 Ctrl + F / B / N / P / A / E 导航（macOS）。
    pub extra_candidate_navigation: bool,

    /// 切换当前中英模式的全 / 半角标点（macOS 只切中文模式的），空串禁用，例如 `ctrl+.`。
    #[serde(with = "super::optional_key_combo")]
    pub toggle_punctuation: Option<KeyCombo>,

    /// 主翻页键以外同时启用的符号键对（macOS、Windows）。
    pub extra_page_keys: PageKeys,

    /// 数字键配这些修饰键：上屏候选的第一个译词。
    pub translation: Modifiers,

    /// 数字键配这些修饰键：上屏候选的第二个译词（候选右侧有两个译词时）。
    pub translation_second: Modifiers,

    /// macOS：回车优先上屏高亮候选的第一条非中文译词；没有可用译词时仍走原回车逻辑。
    pub enter_translation: bool,

    /// 把应用里选中的文字译成学习语言（需要云服务开着）。
    pub translate_selection: KeyCombo,

    /// 组句中按需调用 AI（缺省 Ctrl+Alt+J，macOS 是 ⌃⌥J）。
    pub predict: KeyCombo,

    /// 没在打字时打开符号面板的翻译组（缺省 Ctrl+Alt+Y，macOS 是 ⌃⌥Y；需要云服务开着）。
    pub translate_panel: KeyCombo,

    /// 呼出剪贴板面板（实验功能，Windows 用），见 [`super::ClipboardConfig`]。
    pub clipboard: KeyCombo,

    /// 在简体与上次用的繁体字形之间切换（Windows 用），空串禁用（缺省），例如 `ctrl+shift+f`。
    /// 缺省不开：只要青简是当前输入法，这个组合在所有应用里都会被拿走，编辑器的「在文件中查找」也用它。
    #[serde(with = "super::optional_key_combo")]
    pub toggle_traditional: Option<KeyCombo>,

    /// 组句中切换单字模式（Windows 用，#848）：候选只留单字，再按一次恢复，上屏完自动回到正常候选。
    /// 空串禁用。只在组句时拿走这个组合，没在打字时应用照常收到（缺省 Ctrl + D 不影响书签、复制行）。
    #[serde(with = "super::optional_key_combo")]
    pub single_chars: Option<KeyCombo>,

    /// 组句时上屏高亮候选的键（Windows、Linux 用），缺省不带 Shift 的空格。详见 [`CommitKey`]。
    pub commit_candidate: CommitKey,

    /// 组句时原样上屏敲的字母的键（Windows、Linux 用），缺省不带 Shift 的回车。
    pub commit_raw: CommitKey,
}

impl Default for ShortcutConfig {
    fn default() -> Self {
        // Windows 上 Alt+数字被系统当菜单快捷键截走（TSF 收不到），译词键缺省用 Ctrl；macOS 用 Option。
        #[cfg(windows)]
        let (translation, translation_second) = (Modifiers::CONTROL, Modifiers::SHIFT_CONTROL);
        #[cfg(not(windows))]
        let (translation, translation_second) = (Modifiers::OPTION, Modifiers::SHIFT_OPTION);
        #[cfg(not(any(windows, target_os = "macos")))]
        let translate_selection = KeyCombo {
            modifiers: Modifiers {
                option: true,
                shift: true,
                ..Modifiers::default()
            },
            key: 't',
        };
        #[cfg(any(windows, target_os = "macos"))]
        let translate_selection = KeyCombo::TRANSLATE_DEFAULT;
        Self {
            mode: ModeKeys::default(),
            switch_mode: SwitchKeys::default(),
            highlight_keys: HighlightKeys::UpDown,
            extra_candidate_navigation: false,
            toggle_punctuation: None,
            extra_page_keys: PageKeys::default(),
            translation,
            translation_second,
            enter_translation: false,
            translate_selection,
            predict: KeyCombo::PREDICT_DEFAULT,
            translate_panel: KeyCombo::TRANSLATE_PANEL_DEFAULT,
            clipboard: KeyCombo::CLIPBOARD_DEFAULT,
            toggle_traditional: None,
            single_chars: Some(KeyCombo::SINGLE_CHARS_DEFAULT),
            commit_candidate: CommitKey::SPACE,
            commit_raw: CommitKey::ENTER,
        }
    }
}

impl ShortcutConfig {
    /// 两个上屏键；配成同一个键时退回缺省，不做一半（读手改的配置用，设置程序录的时候就不让撞）。
    pub fn commit_keys(&self) -> CommitKeys {
        if self.commit_candidate == self.commit_raw {
            CommitKeys::default()
        } else {
            CommitKeys {
                candidate: self.commit_candidate,
                raw: self.commit_raw,
            }
        }
    }

    /// 两组译词修饰键；两组相同或有一组为空时整个退回缺省，不做一半。
    pub fn translation_keys(&self) -> (Modifiers, Modifiers) {
        if self.translation == self.translation_second
            || self.translation.is_empty()
            || self.translation_second.is_empty()
        {
            let default = Self::default();
            (default.translation, default.translation_second)
        } else {
            (self.translation, self.translation_second)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enter_translation_defaults_off_and_round_trips() {
        let old: ShortcutConfig = toml::from_str("expression = 'i'").unwrap();
        assert!(!old.enter_translation);
        for enabled in [false, true] {
            let config: ShortcutConfig =
                toml::from_str(&format!("enter_translation = {enabled}")).unwrap();
            let restored: ShortcutConfig =
                toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
            assert_eq!(restored.enter_translation, enabled);
            assert_eq!(restored.commit_raw, old.commit_raw);
            assert_eq!(restored.commit_candidate, old.commit_candidate);
        }
        assert!(toml::from_str::<ShortcutConfig>("enter_translation = 'yes'").is_err());
    }

    #[test]
    fn optional_punctuation_and_extra_page_keys_round_trip() {
        let default: ShortcutConfig = toml::from_str("").unwrap();
        assert_eq!(default.toggle_punctuation, None);
        assert_eq!(default.extra_page_keys.step('.'), None);
        let config: ShortcutConfig =
            toml::from_str("toggle_punctuation = 'ctrl+.'\nextra_page_keys = ['[]', ',.', '-=']")
                .unwrap();
        assert_eq!(config.toggle_punctuation.unwrap().key, '.');
        for key in ['[', ',', '-'] {
            assert_eq!(config.extra_page_keys.step(key), Some(-1));
        }
        for key in [']', '.', '='] {
            assert_eq!(config.extra_page_keys.step(key), Some(1));
        }
        let round_trip: ShortcutConfig =
            toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
        assert_eq!(round_trip, config);
        assert_eq!(
            toml::from_str::<ShortcutConfig>("toggle_punctuation = ''")
                .unwrap()
                .toggle_punctuation,
            None
        );
        assert!(toml::from_str::<ShortcutConfig>("extra_page_keys = ['ab']").is_err());
        assert!(toml::from_str::<ShortcutConfig>("toggle_punctuation = 'ctrl+unknown'").is_err());
    }

    #[test]
    fn traditional_shortcut_roundtrips_and_can_be_disabled() {
        let default: ShortcutConfig = toml::from_str("").unwrap();
        assert_eq!(default.toggle_traditional, None);
        for text in ["", "ctrl+shift+f", "ctrl+alt+g"] {
            let source = format!("toggle_traditional = {text:?}");
            let config: ShortcutConfig = toml::from_str(&source).unwrap();
            let restored: ShortcutConfig =
                toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
            assert_eq!(restored.toggle_traditional, config.toggle_traditional);
            assert_eq!(config.toggle_traditional.is_none(), text.is_empty());
        }
        assert!(toml::from_str::<ShortcutConfig>("toggle_traditional = \"f\"").is_err());
    }

    #[test]
    fn single_chars_shortcut_defaults_on_and_can_be_disabled() {
        let default: ShortcutConfig = toml::from_str("").unwrap();
        assert_eq!(default.single_chars, Some(KeyCombo::SINGLE_CHARS_DEFAULT));
        for text in ["", "ctrl+d", "ctrl+shift+d"] {
            let config: ShortcutConfig =
                toml::from_str(&format!("single_chars = {text:?}")).unwrap();
            let restored: ShortcutConfig =
                toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
            assert_eq!(restored.single_chars, config.single_chars);
            assert_eq!(config.single_chars.is_none(), text.is_empty());
        }
    }

    #[test]
    fn old_files_without_modifier_keys_still_parse_and_get_defaults() {
        // 缺省值分平台（Windows 用 Ctrl 系、其余用 Option 系），断言跟着平台的 Default 走
        let default = ShortcutConfig::default();
        let parsed: ShortcutConfig = toml::from_str("expression = \"i\"\n").unwrap();
        assert_eq!(parsed.mode.expression, 'i');
        assert_eq!(parsed.switch_mode, SwitchKeys::default());
        assert_eq!(
            parsed.translation_keys(),
            (default.translation, default.translation_second)
        );
        let same: ShortcutConfig =
            toml::from_str("translation = \"option\"\ntranslation_second = \"option\"\n").unwrap();
        assert_eq!(
            same.translation_keys(),
            (default.translation, default.translation_second)
        );
        let swapped: ShortcutConfig =
            toml::from_str("translation = \"control+option\"\ntranslation_second = \"option\"\n")
                .unwrap();
        assert_eq!(swapped.translation_keys().1, Modifiers::OPTION);
    }

    #[test]
    fn switch_mode_parses_and_falls_back_to_the_platform_default() {
        let parsed: ShortcutConfig = toml::from_str("switch_mode = [\"ctrl\"]\n").unwrap();
        assert!(parsed.switch_mode.control && !parsed.switch_mode.shift);
        let off: ShortcutConfig = toml::from_str("switch_mode = \"none\"\n").unwrap();
        assert_eq!(off.switch_mode, crate::SwitchKeys::NONE);
        let missing: ShortcutConfig = toml::from_str("").unwrap();
        assert_eq!(missing.switch_mode, SwitchKeys::default());
    }

    #[test]
    fn extra_candidate_navigation_defaults_and_round_trips() {
        let old: ShortcutConfig = toml::from_str("expression = 'i'").unwrap();
        assert!(!old.extra_candidate_navigation);
        for enabled in [false, true] {
            let config: ShortcutConfig =
                toml::from_str(&format!("extra_candidate_navigation = {enabled}")).unwrap();
            assert_eq!(config.extra_candidate_navigation, enabled);
            let restored: ShortcutConfig =
                toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
            assert_eq!(restored.extra_candidate_navigation, enabled);
        }
        assert!(toml::from_str::<ShortcutConfig>("extra_candidate_navigation = 'yes'").is_err());
    }

    #[test]
    fn highlight_keys_parses_and_defaults_to_updown() {
        let parsed: ShortcutConfig = toml::from_str("highlight_keys = \"leftright\"\n").unwrap();
        assert_eq!(parsed.highlight_keys, HighlightKeys::LeftRight);
        // 老配置没有这一项，行为保持以前：↑ / ↓ 移高亮
        let missing: ShortcutConfig = toml::from_str("").unwrap();
        assert_eq!(missing.highlight_keys, HighlightKeys::UpDown);
    }
}
