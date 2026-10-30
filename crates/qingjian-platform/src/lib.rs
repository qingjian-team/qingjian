//! 平台层共用的、与具体窗口系统无关的部分：配置文件，以及将来 Core 与壳之间的协议类型。
//!
//! 这里的类型必须可序列化：macOS / Linux 上 Core 与壳同进程，Windows 上 Core 在独立
//! Server 进程，同一套类型两边都用。

pub mod aux_schemes;
pub mod candidate_menu;
pub mod code_schemes;
pub mod code_tables;
mod config;
pub mod dirs;
mod error;
pub mod extra_dictionaries;
mod i18n;
pub mod logs;
pub mod model_words;
/// Windows 命名管道的对端身份：管道名按用户 SID 派生，两端互验令牌。仅 Windows。
#[cfg(windows)]
pub mod pipe;
pub mod protocol;
pub mod resources;
mod scheme;
pub mod shuangpin_schemes;

pub use config::{
    Appearance, AppsConfig, AuxCodeConfig, CandidateRenderer, CodeRules, CommitKey, CommitKeys,
    CommitRoute, Config, DEFAULT_DOMAINS, DEFAULT_ENGLISH_CANDIDATES_ON,
    DEFAULT_ENGLISH_CANDIDATES_ON_LINUX, DEFAULT_ENGLISH_CANDIDATES_ON_MACOS,
    DEFAULT_ENGLISH_CANDIDATES_ON_WINDOWS, DEFAULT_KEY_HINT_OPACITY, DEFAULT_PAGE_KEYS,
    DEFAULT_THEME, DictionariesConfig, EmojiCount, FontSize, GeneralConfig, HighlightKeys,
    KeyCombo, KeyHintMode, LEARNING_LANGUAGE_OFF, LayoutMode, LocalModel, LocalModelConfig,
    LogLevel, MAX_PAGE_SIZE, Modifiers, NumpadDigit, PAGE_KEY_OPTIONS, PageKeys, PreeditMode,
    Scheme, ShiftLetter, ShortcutConfig, SwitchKey, SwitchKeys, UI_LANGUAGE_DEFAULT,
    UI_LANGUAGE_SYSTEM, UpdateChannel, UpdateConfig, ZhiyiTier, export_phrases, export_settings,
    import_phrases, import_settings, scheme_label, scheme_name,
};
pub use error::ConfigError;
pub use scheme::{SchemeMeta, SchemeSlot};
