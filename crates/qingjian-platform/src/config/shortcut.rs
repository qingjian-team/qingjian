//! 前缀模式与平台快捷键配置。

use qingjian_core::ModeKeys;
use serde::{Deserialize, Serialize};

use super::ModeSwitch;
use super::key_combo::KeyCombo;
use super::modifiers::Modifiers;

/// 配置文件 `[shortcut]` 分节：前缀模式键（Core 的 [`ModeKeys`]）加壳层的修饰键组合。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShortcutConfig {
    /// 表达式 / 问字模式键，键名与以前一样直接在分节下（`expression` / `question`）。
    #[serde(flatten)]
    pub mode: ModeKeys,

    /// 数字键配这些修饰键：上屏候选的第一个译词。
    pub translation: Modifiers,

    /// 数字键配这些修饰键：上屏候选的第二个译词（候选右侧有两个译词时）。
    pub translation_second: Modifiers,

    /// 把应用里选中的文字译成学习语言（需要云服务开着）。
    pub translate_selection: KeyCombo,

    /// macOS 中英文切换：单击 Shift，或修饰键加字母，不能与翻译选中文字相同。
    pub mode_switch: ModeSwitch,

    /// 数字键配这些修饰键：删掉候选（用户词整个删掉，词库词清掉对它的学习）。
    pub delete_candidate: Modifiers,
}

impl Default for ShortcutConfig {
    fn default() -> Self {
        // Windows 上 Alt+数字被系统当菜单快捷键截走（TSF 收不到），译词键缺省用 Ctrl；macOS 用 Option。
        #[cfg(windows)]
        let (translation, translation_second) = (Modifiers::CONTROL, Modifiers::SHIFT_CONTROL);
        #[cfg(not(windows))]
        let (translation, translation_second) = (Modifiers::OPTION, Modifiers::SHIFT_OPTION);
        Self {
            mode: ModeKeys::default(),
            translation,
            translation_second,
            translate_selection: KeyCombo::TRANSLATE_DEFAULT,
            mode_switch: ModeSwitch::default(),
            delete_candidate: Modifiers::SHIFT,
        }
    }
}

impl ShortcutConfig {
    /// 支持单击 Shift 或修饰键加字母，避免抢占候选序号；无效或与翻译键冲突时禁用。
    /// 手工配置与设置窗口使用同一规则，不能让冲突键触发错误动作。
    pub fn mode_switch_keys(&self) -> Option<ModeSwitch> {
        self.mode_switch
            .combo()
            .is_none_or(|combo| {
                combo.key.is_ascii_alphabetic()
                    && !combo.modifiers.is_empty()
                    && combo != self.translate_selection
            })
            .then_some(self.mode_switch)
    }

    /// 删候选的修饰键；为空或与任一组译词键撞了就退回缺省。
    pub fn delete_keys(&self) -> Modifiers {
        let (first, second) = self.translation_keys();
        if self.delete_candidate.is_empty()
            || self.delete_candidate == first
            || self.delete_candidate == second
        {
            Self::default().delete_candidate
        } else {
            self.delete_candidate
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
    fn mode_switch_accepts_a_single_shift() {
        let config: ShortcutConfig = toml::from_str("mode_switch = \"shift\"").unwrap();
        assert_eq!(config.mode_switch.key_string(), "shift");
        assert!(config.mode_switch_keys().is_some());
    }

    #[test]
    fn old_files_without_modifier_keys_still_parse_and_get_defaults() {
        let parsed: ShortcutConfig = toml::from_str("expression = \"i\"\n").unwrap();
        assert_eq!(parsed.mode.expression, 'i');
        assert_eq!(
            parsed.translation_keys(),
            (Modifiers::OPTION, Modifiers::SHIFT_OPTION)
        );
        let same: ShortcutConfig =
            toml::from_str("translation = \"option\"\ntranslation_second = \"option\"\n").unwrap();
        assert_eq!(
            same.translation_keys(),
            (Modifiers::OPTION, Modifiers::SHIFT_OPTION)
        );
        let swapped: ShortcutConfig =
            toml::from_str("translation = \"control+option\"\ntranslation_second = \"option\"\n")
                .unwrap();
        assert_eq!(swapped.translation_keys().1, Modifiers::OPTION);
    }

    #[test]
    fn delete_keys_fall_back_when_clashing_with_translation_keys() {
        let parsed: ShortcutConfig = toml::from_str("").unwrap();
        assert_eq!(parsed.delete_keys(), Modifiers::SHIFT);
        let clash: ShortcutConfig = toml::from_str("delete_candidate = \"option\"\n").unwrap();
        assert_eq!(clash.delete_keys(), Modifiers::SHIFT);
        let custom: ShortcutConfig =
            toml::from_str("delete_candidate = \"control+shift\"\n").unwrap();
        assert!(custom.delete_keys().control);
    }

    #[test]
    fn mode_switch_defaults_round_trip_and_conflicts() {
        let mut parsed: ShortcutConfig = toml::from_str("").unwrap();
        assert_eq!(parsed.mode_switch_keys(), Some(ModeSwitch::Shift));
        parsed.mode_switch = "control+shift+e".parse().unwrap();
        let saved = toml::to_string(&parsed).unwrap();
        assert_eq!(toml::from_str::<ShortcutConfig>(&saved).unwrap(), parsed);
        parsed.mode_switch = ModeSwitch::Combo(parsed.translate_selection);
        assert_eq!(parsed.mode_switch_keys(), None);
        parsed.mode_switch = "option+1".parse().unwrap();
        assert_eq!(parsed.mode_switch_keys(), None);
    }
}
