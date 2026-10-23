use qingjian_core::ModeKeys;
use serde::{Deserialize, Serialize};

use super::PageKeys;
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

    /// 切换当前中英模式的全 / 半角标点（macOS 只切中文模式的），空串禁用，例如 `ctrl+.`。
    #[serde(with = "super::optional_key_combo")]
    pub toggle_punctuation: Option<KeyCombo>,

    /// 主翻页键以外同时启用的符号键对（macOS、Windows）。
    pub extra_page_keys: PageKeys,

    /// 数字键配这些修饰键：上屏候选的第一个译词。
    pub translation: Modifiers,

    /// 数字键配这些修饰键：上屏候选的第二个译词（候选右侧有两个译词时）。
    pub translation_second: Modifiers,

    /// 把应用里选中的文字译成学习语言（需要云服务开着）。
    pub translate_selection: KeyCombo,

    /// 组句中按需调用 AI（缺省 Ctrl+Alt+J，macOS 是 ⌃⌥J）。
    pub predict: KeyCombo,
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
            toggle_punctuation: None,
            extra_page_keys: PageKeys::default(),
            translation,
            translation_second,
            translate_selection,
            predict: KeyCombo::PREDICT_DEFAULT,
        }
    }
}

impl ShortcutConfig {
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
    fn switch_mode_parses_and_defaults_to_shift() {
        let parsed: ShortcutConfig = toml::from_str("switch_mode = [\"ctrl\"]\n").unwrap();
        assert!(parsed.switch_mode.control && !parsed.switch_mode.shift);
        let off: ShortcutConfig = toml::from_str("switch_mode = \"none\"\n").unwrap();
        assert_eq!(off.switch_mode, crate::SwitchKeys::NONE);
        let missing: ShortcutConfig = toml::from_str("").unwrap();
        assert_eq!(missing.switch_mode, SwitchKeys::default());
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
