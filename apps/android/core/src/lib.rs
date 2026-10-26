//! Android 输入法的 Rust 侧：装配 Engine、把软键盘的操作翻成 Engine 调用、经 JNI 交给 Kotlin 壳。
//!
//! Android 的输入法跑在自己的进程里，没有 Server / 薄壳之分，Engine 直接装在这个 `.so` 里。

mod appearance;
mod assembly;
mod cloud;
mod error;
mod frame;
mod handle;
mod jni;
mod options;
mod session;
mod store;
mod strip;
mod voice;

pub use assembly::{assemble, assemble_translation};
pub use cloud::{CloudSettings, test_connection};
pub use error::AndroidError;
pub use frame::{Frame, FrameCandidate, FrameSense};
pub use options::InputOptions;
pub use session::Session;
pub use store::{
    CatalogSection, Choice, ClipboardItem, ClipboardRequest, CodeView, DictionaryEntry,
    DictionaryPage, DictionaryRequest, PhrasePage, PhraseRequest, PhraseRules, SettingChoices,
    ShuangpinChoice, ShuangpinKeycap, StatsCount, StatsDay, StatsDetail, StatsLevel, StatsSummary,
    StatsVocabulary, VocabularyChange, VocabularyFilter, VocabularyPage, VocabularyRequest,
    VocabularyWord, catalog, clear_learning, clipboard, clipboard_add_image, code_view,
    dictionaries, export_files, phrase_rules, phrases, setting_choices, shuangpin_choices,
    shuangpin_keycaps, stats, stats_detail, vocabulary,
};
pub use strip::{StripHit, StripMeta, StripPainter};
pub use voice::{VoiceSettings, test_voice};
