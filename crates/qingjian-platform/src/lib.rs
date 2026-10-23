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
pub mod logs;
pub mod protocol;
pub mod resources;
mod scheme_meta;
mod scheme_slot;
pub mod shuangpin_schemes;

pub use config::{
    Appearance, AppsConfig, AuxCodeConfig, CandidateRenderer, CodeRules, Config, DEFAULT_DOMAINS,
    DEFAULT_ENGLISH_CANDIDATES_ON, DEFAULT_ENGLISH_CANDIDATES_ON_LINUX,
    DEFAULT_ENGLISH_CANDIDATES_ON_MACOS, DEFAULT_ENGLISH_CANDIDATES_ON_WINDOWS,
    DEFAULT_KEY_HINT_OPACITY, DEFAULT_PAGE_KEYS, DEFAULT_THEME, DictionariesConfig, EmojiCount,
    FontSize, GeneralConfig, HighlightKeys, KeyCombo, KeyHintMode, LEARNING_LANGUAGE_OFF,
    LayoutMode, LocalModel, LocalModelConfig, LogLevel, MAX_PAGE_SIZE, Modifiers, PAGE_KEY_OPTIONS,
    PageKeys, PreeditMode, Scheme, ShiftLetter, ShortcutConfig, SwitchKey, SwitchKeys,
    UpdateChannel, UpdateConfig, ZhiyiTier, scheme_label, scheme_name,
};
pub use error::ConfigError;
pub use scheme_meta::SchemeMeta;
pub use scheme_slot::SchemeSlot;
