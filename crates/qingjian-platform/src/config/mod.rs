mod appearance;
mod apps;
mod auto_phrase;
mod aux_code;
mod candidate_renderer;
mod clipboard;
mod code_rules;
mod commit_key;
mod dictionaries;
mod emoji_count;
mod font_size;
mod general;
mod highlight_keys;
mod io;
mod key_combo;
mod key_hint_mode;
mod layout_mode;
mod legacy_traditional;
mod log_level;
mod model;
mod modifiers;
mod numpad_digit;
mod optional_key_combo;
mod page_keys;
mod phrases;
mod preedit_mode;
mod retired;
mod scheme;
mod shift_letter;
mod shortcut;
mod status_bar;
mod switch_key;
mod template;
mod third_input_method;
mod transfer;
mod update;

use std::collections::BTreeMap;

use qingjian_core::FuzzyRules;
use qingjian_predict::PredictConfig;
use serde::{Deserialize, Serialize};

pub use appearance::Appearance;
pub use apps::{
    AppsConfig, DEFAULT_ENGLISH_CANDIDATES_ON, DEFAULT_ENGLISH_CANDIDATES_ON_LINUX,
    DEFAULT_ENGLISH_CANDIDATES_ON_MACOS, DEFAULT_ENGLISH_CANDIDATES_ON_WINDOWS,
};
pub use auto_phrase::AutoPhraseConfig;
pub use aux_code::AuxCodeConfig;
pub use candidate_renderer::CandidateRenderer;
pub use clipboard::{
    CLIPBOARD_HISTORY_LIMITS, ClipboardConfig, ClipboardEdge, DEFAULT_CLIPBOARD_EXCLUDED_APPS,
};
pub use code_rules::CodeRules;
pub use commit_key::{CommitKey, CommitKeys, CommitRoute};
pub use dictionaries::{DEFAULT_DOMAINS, DictionariesConfig};
pub use emoji_count::EmojiCount;
pub use font_size::FontSize;
pub use general::{
    DEFAULT_KEY_HINT_OPACITY, DEFAULT_PAGE_KEYS, DEFAULT_THEME, GeneralConfig,
    LEARNING_LANGUAGE_OFF, MAX_PAGE_SIZE, PAGE_KEY_OPTIONS, UI_LANGUAGE_DEFAULT,
    UI_LANGUAGE_SYSTEM,
};
pub use highlight_keys::HighlightKeys;
pub use key_combo::KeyCombo;
pub use key_hint_mode::KeyHintMode;
pub use layout_mode::LayoutMode;
pub use legacy_traditional::LegacyTraditional;
pub use log_level::LogLevel;
pub use model::{LocalModel, LocalModelConfig, ZhiyiTier};
pub use modifiers::Modifiers;
pub use numpad_digit::NumpadDigit;
pub use page_keys::PageKeys;
pub use phrases::{export_phrases, import_phrases};
pub use preedit_mode::PreeditMode;
pub use scheme::{Scheme, scheme_label, scheme_name};
pub use shift_letter::ShiftLetter;
pub use shortcut::ShortcutConfig;
pub use status_bar::StatusBarConfig;
pub use switch_key::{SwitchKey, SwitchKeys};
pub use template::TEMPLATE;
pub use third_input_method::ThirdInputMethod;
pub use transfer::{export_settings, import_settings};
pub use update::{UpdateChannel, UpdateConfig};

// 仅 crate 内复用：落盘辅助函数；模板注释常量只供配置单测对照。
pub(crate) use io::write_file;
#[cfg(test)]
pub(crate) use io::{APPEARANCE_COMMENT, THEME_COMMENT};

/// 用户配置文件（TOML）。所有平台同一份格式，缺省值全部在各分节的 `Default` 里。
///
/// 配置文件是唯一事实源：菜单、设置窗口、手改文件三个入口都只写这个文件，再由壳热加载。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// 五笔连续组词的开关与最大词长，不改变拼音、双拼的学习规则。
    pub auto_phrase: AutoPhraseConfig,
    /// 常规：学习语言、每页候选数、翻页键、外观。
    pub general: GeneralConfig,

    /// 自定义短语；保存和读取均检查位置冲突。读的时候写错的条目只跳过它自己（见 `phrases`）。
    #[serde(deserialize_with = "phrases::deserialize")]
    pub custom_phrases: Vec<qingjian_core::CustomPhrase>,

    /// 快捷键：前缀模式键（表达式 / 问字）与上屏译词的修饰键组合。
    pub shortcut: ShortcutConfig,

    /// 模糊音开关。
    pub fuzzy: FuzzyRules,

    /// 附加词库开关。
    pub dictionaries: DictionariesConfig,

    /// 辅码码表开关。
    pub aux_code: AuxCodeConfig,

    /// 按应用改行为（哪些应用里英文模式不给候选）。
    pub apps: AppsConfig,

    /// 云联想。
    pub predict: PredictConfig,

    /// 悬浮状态条（桌面上常驻、可拖动的中 / 英浮窗）。
    pub status_bar: StatusBarConfig,

    /// 本地整句模型。
    pub model: LocalModelConfig,

    /// 青简剪贴板（实验功能）。
    pub clipboard: ClipboardConfig,

    /// 检查更新。
    pub update: UpdateConfig,

    /// 各形码方案的按键规则（`[code.<id>]`），按方案 id；没写的方案按它自己的 `scheme.toml`。
    pub code: BTreeMap<String, CodeRules>,
}

#[cfg(test)]
mod tests;
