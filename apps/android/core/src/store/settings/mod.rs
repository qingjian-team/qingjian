//! 键盘设置页的选项清单（模糊音、学习语言、繁体字形、剪贴板条数）与缺省值：名字从 Core 取，与 PC 设置程序同一份。

mod choice;
mod choices;

pub use choice::Choice;
pub use choices::SettingChoices;

use qingjian_core::{FuzzyRules, LEARNING_LANGUAGES, TraditionalVariant};
use qingjian_learning::DEFAULT_CLIPBOARD_LIMIT;

use crate::options::{DEFAULT_DOMAINS, DEFAULT_LEARNING_LANGUAGE, DEFAULT_TRADITIONAL_VARIANT};

/// 剪贴板条数的几档，缺省那档在里面。
const CLIPBOARD_LIMITS: [usize; 4] = [100, 500, DEFAULT_CLIPBOARD_LIMIT, 5000];

pub fn setting_choices() -> SettingChoices {
    let fuzzy = FuzzyRules::NAMES
        .into_iter()
        .map(|key| Choice {
            key,
            label: FuzzyRules::label(key),
        })
        .collect();
    let learning_languages = LEARNING_LANGUAGES
        .into_iter()
        .map(|(key, label)| Choice {
            key,
            label: label.to_owned(),
        })
        .collect();
    let traditional_variants = TraditionalVariant::ALL
        .into_iter()
        .filter(|variant| *variant != TraditionalVariant::Off)
        .map(|variant| Choice {
            key: variant.key(),
            label: variant.short_label().to_owned(),
        })
        .collect();
    SettingChoices {
        fuzzy,
        learning_languages,
        default_learning_language: DEFAULT_LEARNING_LANGUAGE,
        traditional_variants,
        default_traditional_variant: DEFAULT_TRADITIONAL_VARIANT.key(),
        default_domains: DEFAULT_DOMAINS.to_vec(),
        clipboard_limits: CLIPBOARD_LIMITS.to_vec(),
        default_clipboard_limit: DEFAULT_CLIPBOARD_LIMIT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_match_what_the_keyboard_settings_showed() {
        let choices = setting_choices();
        assert_eq!(choices.fuzzy.len(), 9);
        assert_eq!(choices.fuzzy[0].key, "z_zh");
        assert_eq!(choices.fuzzy[0].label, "z = zh");
        let languages: Vec<&str> = choices.learning_languages.iter().map(|c| c.key).collect();
        assert_eq!(languages, ["en", "ja", "es", "vi", "de", "off"]);
        assert_eq!(choices.learning_languages[5].label, "不显示");
        let variants: Vec<(&str, &str)> = choices
            .traditional_variants
            .iter()
            .map(|c| (c.key, c.label.as_str()))
            .collect();
        assert_eq!(
            variants,
            [
                ("taiwan", "台湾"),
                ("hongkong", "香港"),
                ("standard", "通用")
            ]
        );
        assert_eq!(choices.default_traditional_variant, "taiwan");
        assert_eq!(choices.default_learning_language, "en");
        assert_eq!(choices.default_domains, ["idioms"]);
        assert_eq!(choices.clipboard_limits, [100, 500, 1000, 5000]);
        assert!(
            choices
                .clipboard_limits
                .contains(&choices.default_clipboard_limit)
        );
    }
}
