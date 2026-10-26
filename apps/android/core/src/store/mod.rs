//! 不经输入会话的数据读写：面板目录、剪贴板、常用语、代码显示、词库、学习数据的导出与清空、键盘设置的选项、双拼方案、首页统计、生词本。
//! 都不要句柄（不碰引擎），输入法与 App 两边都能调。

pub(crate) mod catalog;
mod clipboard;
mod code;
pub(crate) mod dictionaries;
pub(crate) mod learning;
mod phrases;
mod settings;
mod shuangpin;
pub(crate) mod stats;
pub(crate) mod vocabulary;

pub use catalog::{CatalogSection, catalog};
pub use clipboard::{ClipboardItem, ClipboardRequest, clipboard, clipboard_add_image};
pub use code::{CodeView, code_view};
pub use dictionaries::{DictionaryEntry, DictionaryPage, DictionaryRequest, dictionaries};
pub use learning::{clear as clear_learning, export_files};
pub use phrases::{PhrasePage, PhraseRequest, PhraseRules, phrase_rules, phrases};
pub use settings::{Choice, SettingChoices, setting_choices};
pub use shuangpin::{ShuangpinChoice, ShuangpinKeycap, shuangpin_choices, shuangpin_keycaps};
pub use stats::{
    StatsCount, StatsDay, StatsDetail, StatsLevel, StatsSummary, StatsVocabulary, stats,
    stats_detail,
};
pub use vocabulary::{
    VocabularyChange, VocabularyFilter, VocabularyPage, VocabularyRequest, VocabularyWord,
    vocabulary,
};
