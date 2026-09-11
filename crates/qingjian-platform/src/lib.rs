//! 平台层共用的、与具体窗口系统无关的部分：配置文件，以及将来 Core 与壳之间的协议类型。
//!
//! 这里的类型必须可序列化：macOS / Linux 上 Core 与壳同进程，Windows 上 Core 在独立
//! Server 进程，同一套类型两边都用。

mod config;
mod error;
pub mod extra_dictionaries;
pub mod protocol;

pub use config::{
    AppsConfig, Config, DEFAULT_DOMAINS, DEFAULT_ENGLISH_CANDIDATES_OFF, DEFAULT_PAGE_KEYS,
    DictionariesConfig, GeneralConfig, KeyCombo, LayoutMode, LogLevel, MAX_PAGE_SIZE, Modifiers,
    PAGE_KEY_OPTIONS, PreeditMode, ShortcutConfig, ThemeMode,
};
pub use error::ConfigError;
