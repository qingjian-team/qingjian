//! 青简输入法内核。
//!
//! 平台无关：词库、拼音解析、候选生成、排序、学习与翻译的接口全部在这里。
//! 平台层（IMK / TSF / IBus-Fcitx）只负责把按键喂给 [`Engine`]、把候选画出来。
//! 判断标准：换掉 IMK 换成 TSF，不应该需要改这里的任何一行。

pub mod calculation;
pub mod candidate;
pub mod chaizi;
pub mod code;
pub mod code_scheme;
pub mod composition;
pub mod correction;
pub mod custom_phrase;
pub mod email;
pub mod emoji;
pub mod engine;
pub mod english;
pub mod fuzzy;
pub mod history;
mod i18n;
pub mod japanese;
pub mod nine_key;
pub mod panel;
pub mod parser;
pub mod punctuation;
pub mod ranking;
pub mod root_chart;
pub mod sentence;
pub mod shortcut;
pub mod shuangpin;
pub mod spacing;
pub mod storage;
pub mod swipe;
pub mod traditional;
pub mod vietnamese;
pub mod voice;
pub mod zhuyin;

pub use custom_phrase::CustomPhrase;

pub use calculation::{CalculationCompletion, calculation_completions, may_end_calculation};
pub use candidate::{
    Candidate, CandidateKind, CandidateLayout, CandidateList, Cell, GRID_ROWS, Grid,
    LEARNING_LANGUAGE_OFF, Language, MAX_CELL_EMS, PartOfSpeech, Sense, Translation,
    learning_languages,
};
pub use code_scheme::CodeScheme;
pub use composition::Composition;
pub use correction::Correction;
pub use email::{EmailCompletion, email_completions};
pub use emoji::{EmojiTable, skin_tone_variants};
pub use engine::{
    AnnotationReport, AuxSegment, BOOKS, Book, CharTable, ChoiceModel, ChoiceNote, ChoiceScore,
    ChoiceTier, CloudWord, CommitEntry, DEFAULT_AUX_CODE_KEY, ENGLISH_CANDIDATES_MAX, Encounter,
    Engine, EngineSession, FRESH_UNTIL, FilledGloss, Forgotten, GlossFiller, INPUT_LOG_VERSION,
    InputLogEntry, InputLogger, InputSource, Learner, LevelCount, MarkedKind, MarkedSegment,
    ModeKeys, NEURAL_MARGIN, NEURAL_WEIGHT, NoGlossFiller, NoInputLogger, NoLearner, NoPredictor,
    NoTranslator, NoUsageMeter, NoVocabularyTracker, Prediction, PredictionKind, PredictionPolicy,
    PredictionRequest, Predictor, QUESTION_PREFIX, Query, RESCORE_CONTEXT_CHARS, RawPreedit,
    SurroundingText, Timings, Translator, Usage, UsageMeter, UsageSummary, VocabularySummary,
    VocabularyTracker, Wake, book_scale, is_valid_aux_code_key,
};
pub use fuzzy::FuzzyRules;
pub use history::{DEFAULT_FRESH_SECONDS, InputHistory};
pub use japanese::RomajiComposer;
pub use panel::{
    NoPanelHistory, PANEL_COLUMNS, PanelGroup, PanelHistory, PanelSection, PanelUse, SymbolPanel,
    catalog_sections, common_items,
};
pub use parser::{ParseError, Segmentation};
pub use punctuation::Punctuation;
pub use qingjian_dictionary as dictionary;
pub use shuangpin::{Layout as ShuangpinLayout, Scheme as ShuangpinScheme};
pub use swipe::{CornerDecoder, ShapeDecoder, SwipeDecoder, SwipeKey, SwipeLexicon, SwipePoint};
pub use traditional::{Traditional, TraditionalVariant};
pub use vietnamese::{
    TelexComposer, complete as complete_vietnamese, suggest as suggest_vietnamese,
};
pub use voice::{NoTranscriber, Transcriber, TranscriptEvent};
