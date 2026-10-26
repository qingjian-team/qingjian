//! 录快捷键：哪几项能录、按下的键能不能用（缺修饰键、跟别的撞了），以及各平台给人看的写法。

use bamboo_slip::{KeyCode, KeyPress};
use qingjian_platform::{Config, KeyCombo, Modifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shortcut {
    Translation,
    TranslationSecond,
    TranslateSelection,
    Predict,
    /// 辅码触发键（单个符号，不带修饰键）。
    AuxCodeKey,
}

impl Shortcut {
    #[cfg(test)]
    pub const ALL: [Self; 5] = [
        Self::Translation,
        Self::TranslationSecond,
        Self::TranslateSelection,
        Self::Predict,
        Self::AuxCodeKey,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Translation => "上屏第一个译词",
            Self::TranslationSecond => "上屏第二个译词",
            Self::TranslateSelection => "翻译选中文字",
            Self::Predict => "按需 AI 联想",
            Self::AuxCodeKey => "触发键",
        }
    }

    /// 配置文件里的分节与键。
    pub fn config_key(self) -> (&'static str, &'static str) {
        match self {
            Self::Translation => ("shortcut", "translation"),
            Self::TranslationSecond => ("shortcut", "translation_second"),
            Self::TranslateSelection => ("shortcut", "translate_selection"),
            Self::Predict => ("shortcut", "predict"),
            Self::AuxCodeKey => ("general", "aux_code_key"),
        }
    }

    /// 录制时的提示。
    pub fn prompt(self) -> &'static str {
        match self {
            Self::Translation | Self::TranslationSecond => "按住修饰键再按个数字",
            Self::TranslateSelection | Self::Predict => "按下修饰键 + 字母",
            Self::AuxCodeKey => "按一个符号键",
        }
    }

    /// 现在的值，给人看的写法。
    pub fn current(self, config: &Config) -> String {
        let shortcut = &config.shortcut;
        let (first, second) = shortcut.translation_keys();
        match self {
            Self::Translation => digits(first),
            Self::TranslationSecond => digits(second),
            Self::TranslateSelection => combo_label(shortcut.translate_selection),
            Self::Predict => combo_label(shortcut.predict),
            Self::AuxCodeKey => config.general.aux_code_key().to_string(),
        }
    }

    /// 按下的键能用就给出要写进配置的值，不能用给出原因（录制接着等）。
    pub fn accept(self, press: &KeyPress, config: &Config) -> Result<String, String> {
        let modifiers = modifiers(press);
        match self {
            Self::AuxCodeKey => {
                let key = match &press.key {
                    KeyCode::Character(text) if modifiers.is_empty() => single(text),
                    _ => None,
                }
                .ok_or("要不带修饰键的单个符号键")?;
                if qingjian_core::is_valid_aux_code_key(key, config.general.page_keys()) {
                    Ok(key.to_string())
                } else {
                    Err(format!(
                        "「{key}」不行：字母、数字、单引号与翻页键都不能当触发键"
                    ))
                }
            }
            Self::TranslateSelection | Self::Predict => {
                let key = match &press.key {
                    KeyCode::Character(text) => single(text).filter(|key| {
                        key.is_ascii_alphanumeric() || (self == Self::Predict && *key == '.')
                    }),
                    _ => None,
                }
                .ok_or("要修饰键加一个字母或数字")?;
                if modifiers.is_empty() {
                    return Err("要带修饰键".to_owned());
                }
                Ok(KeyCombo {
                    modifiers,
                    key: key.to_ascii_lowercase(),
                }
                .key_string())
            }
            Self::Translation | Self::TranslationSecond => {
                if modifiers.is_empty() {
                    return Err("要按住修饰键".to_owned());
                }
                let (first, second) = config.shortcut.translation_keys();
                let others = [
                    (Self::Translation, first),
                    (Self::TranslationSecond, second),
                ];
                if let Some((other, _)) = others
                    .iter()
                    .find(|(other, keys)| *other != self && *keys == modifiers)
                {
                    return Err(format!("跟「{}」重复了", other.title()));
                }
                Ok(modifiers.key())
            }
        }
    }
}

fn single(text: &str) -> Option<char> {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Some(c),
        _ => None,
    }
}

/// 按键上按住的修饰键，换成配置的写法（Windows 上 command 记的是 Win 键）。
fn modifiers(press: &KeyPress) -> Modifiers {
    Modifiers {
        option: press.modifiers.alt,
        shift: press.modifiers.shift,
        control: press.modifiers.control,
        command: press.modifiers.logo,
    }
}

/// 修饰键的写法：macOS 用符号 ⌃⇧⌥⌘，其他平台写键名。
fn modifiers_label(modifiers: Modifiers) -> String {
    if cfg!(target_os = "macos") {
        return modifiers.label();
    }
    let mut names = Vec::new();
    if modifiers.control {
        names.push("Ctrl");
    }
    if modifiers.shift {
        names.push("Shift");
    }
    if modifiers.option {
        names.push("Alt");
    }
    if modifiers.command {
        names.push("Win");
    }
    names.join(" + ")
}

/// 修饰键配候选序号。
fn digits(modifiers: Modifiers) -> String {
    format!("{} + 序号", modifiers_label(modifiers))
}

fn combo_label(combo: KeyCombo) -> String {
    let key = combo.key.to_ascii_uppercase();
    if cfg!(target_os = "macos") {
        format!("{}{key}", modifiers_label(combo.modifiers))
    } else {
        format!("{} + {key}", modifiers_label(combo.modifiers))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bamboo_slip::Modifiers as Held;

    fn press(key: &str, held: Held) -> KeyPress {
        KeyPress::new(KeyCode::Character(key.to_owned()), held)
    }

    fn alt() -> Held {
        Held {
            alt: true,
            ..Held::default()
        }
    }

    #[test]
    fn modifier_sets_must_be_held_and_distinct() {
        let config = Config::default();
        let (first, _) = config.shortcut.translation_keys();
        assert!(
            Shortcut::Translation
                .accept(&press("1", Held::default()), &config)
                .is_err()
        );
        // 第二组译词键不能跟第一组一样
        let same = Held {
            alt: first.option,
            shift: first.shift,
            control: first.control,
            logo: first.command,
            ..Held::default()
        };
        let clash = Shortcut::TranslationSecond.accept(&press("1", same), &config);
        assert_eq!(clash, Err("跟「上屏第一个译词」重复了".to_owned()));
        // 自己跟自己不算撞
        assert_eq!(
            Shortcut::Translation.accept(&press("1", same), &config),
            Ok(first.key())
        );
    }

    #[test]
    fn translate_selection_needs_a_letter_and_aux_key_a_symbol() {
        let config = Config::default();
        assert_eq!(
            Shortcut::TranslateSelection.accept(&press("T", alt()), &config),
            Ok("option+t".to_owned())
        );
        assert!(
            Shortcut::TranslateSelection
                .accept(&press("t", Held::default()), &config)
                .is_err()
        );
        assert_eq!(
            Shortcut::AuxCodeKey.accept(&press("/", Held::default()), &config),
            Ok("/".to_owned())
        );
        assert!(
            Shortcut::AuxCodeKey
                .accept(&press("a", Held::default()), &config)
                .is_err()
        );
        assert!(
            Shortcut::AuxCodeKey
                .accept(&press("[", Held::default()), &config)
                .is_err()
        );
        assert!(
            Shortcut::AuxCodeKey
                .accept(&press(";", alt()), &config)
                .is_err()
        );
    }

    #[test]
    fn prediction_recorder_accepts_modified_letters_and_period() {
        let config = Config::default();
        assert_eq!(Shortcut::Predict.config_key(), ("shortcut", "predict"));
        assert_eq!(
            Shortcut::Predict.accept(&press("J", alt()), &config),
            Ok("option+j".to_owned())
        );
        assert_eq!(
            Shortcut::Predict.accept(&press(".", alt()), &config),
            Ok("option+.".to_owned())
        );
        assert!(
            Shortcut::Predict
                .accept(&press("j", Held::default()), &config)
                .is_err()
        );
    }
}
