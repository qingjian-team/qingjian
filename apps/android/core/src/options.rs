//! 键盘设置里影响候选的几项：模糊音、中文标点全半角、打拼音时给不给英文词、学习语言、繁体字形、学习开关、附加词库、分次选词留在组句里。Kotlin 按设置整份交过来。

use qingjian_core::{FuzzyRules, Language, TraditionalVariant};
use serde::Deserialize;

/// 名字与 PC 配置文件一致（`[fuzzy]` 的字段、`[general]` 的同名项）。
#[derive(Debug, Deserialize)]
pub struct InputOptions {
    /// 开着的模糊音规则：`z_zh`、`an_ang` 这些（[`FuzzyRules::NAMES`]）。
    #[serde(default)]
    pub fuzzy: Vec<String>,

    /// 中文键盘的标点键出全角（，。）还是半角（,.）。
    pub full_width_punctuation: bool,

    /// 打拼音时也给英文词。
    pub english_in_chinese: bool,

    /// 中文候选总排在英文词前面。
    pub chinese_first: bool,

    /// 学习语言（`en` / `ja` / `es` / `vi` / `de`，`off` 不显示译文）。
    #[serde(default = "default_learning_language")]
    pub learning_language: String,

    /// 功能面板「繁体」开着时出哪一地的字形（`taiwan` / `hongkong` / `standard`），同 PC 的 `traditional_variant`。
    #[serde(default = "default_traditional_variant")]
    pub traditional_variant: TraditionalVariant,

    /// 学习输入习惯（PC 的 `[general] learning`）：关了不再记新的，已学的照常参与排序。
    #[serde(default = "default_true")]
    pub learning: bool,

    /// 打开的随包领域词库（文件名不含扩展名），同 PC 的 `[dictionaries] domains`；缺省只开成语。
    #[serde(default = "default_domains")]
    pub domains: Vec<String>,

    /// 关掉的导入词库（文件名不含扩展名），同 PC 的 `[dictionaries] disabled`。
    #[serde(default)]
    pub disabled_dictionaries: Vec<String>,

    /// 一段拼音分几次选时，选好的词先留在组句里、整段选完才上屏，退格先撤回上一次选择（PC 的 `[general] hold_selections`）。
    #[serde(default = "default_true")]
    pub hold_selections: bool,
}

/// 没选过时的学习语言，同 PC。
pub const DEFAULT_LEARNING_LANGUAGE: &str = "en";

/// 没选过时的繁体字形，同 PC 设置程序「输出字形」开着时的第一档。
pub const DEFAULT_TRADITIONAL_VARIANT: TraditionalVariant = TraditionalVariant::Taiwan;

/// 缺省打开的随包领域词库，同 PC 的 `[dictionaries] domains`：成语四字全拼几乎不歧义。
pub const DEFAULT_DOMAINS: [&str; 1] = ["idioms"];

fn default_learning_language() -> String {
    DEFAULT_LEARNING_LANGUAGE.to_owned()
}

fn default_traditional_variant() -> TraditionalVariant {
    DEFAULT_TRADITIONAL_VARIANT
}

fn default_true() -> bool {
    true
}

fn default_domains() -> Vec<String> {
    DEFAULT_DOMAINS.map(str::to_owned).to_vec()
}

impl InputOptions {
    pub fn fuzzy_rules(&self) -> FuzzyRules {
        let mut rules = FuzzyRules::default();
        for name in &self.fuzzy {
            if !rules.enable(name) {
                tracing::warn!(name, "不认识的模糊音规则");
            }
        }
        rules
    }

    pub fn learning_language(&self) -> Option<Language> {
        learning_language(&self.learning_language)
    }

    /// 繁体开着时用的字形；写成 `off` 的按台湾。
    pub fn traditional_variant(&self) -> TraditionalVariant {
        match self.traditional_variant {
            TraditionalVariant::Off => TraditionalVariant::Taiwan,
            variant => variant,
        }
    }
}

/// 学习语言的代码：`off` 是 `None`；认不出的、写成中文的都按英语，与 PC 一致。
pub fn learning_language(code: &str) -> Option<Language> {
    if code.trim().eq_ignore_ascii_case("off") {
        return None;
    }
    match code.parse() {
        Ok(Language::Chinese) | Err(_) => Some(Language::English),
        Ok(language) => Some(language),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_names_follow_pc_config() {
        let options: InputOptions = serde_json::from_str(
            r#"{"fuzzy":["z_zh","an_ang","bogus"],"full_width_punctuation":true,"english_in_chinese":true,"chinese_first":false}"#,
        )
        .unwrap();
        let rules = options.fuzzy_rules();
        assert!(rules.z_zh && rules.an_ang);
        assert!(!rules.n_l);
        assert_eq!(options.learning_language(), Some(Language::English));
    }

    #[test]
    fn learning_language_off_and_fallback() {
        assert_eq!(learning_language("off"), None);
        assert_eq!(learning_language("ja"), Some(Language::Japanese));
        assert_eq!(learning_language("fr"), Some(Language::English));
        assert_eq!(learning_language("zh"), Some(Language::English));
    }
}
