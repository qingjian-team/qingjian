//! 青简输入法内核。
//!
//! 平台无关：词库、拼音解析、候选生成、排序、学习与翻译的接口全部在这里。
//! 平台层（IMK / TSF / IBus-Fcitx）只负责把按键喂给 [`Engine`]、把候选画出来。
//! 判断标准：换掉 IMK 换成 TSF，不应该需要改这里的任何一行。

pub mod candidate;
pub mod composition;
pub mod correction;
pub mod emoji;
pub mod engine;
pub mod english;
pub mod fuzzy;
pub mod history;
pub mod parser;
pub mod punctuation;
pub mod ranking;
pub mod sentence;
pub mod shortcut;
pub mod shuangpin;
pub mod storage;

pub use candidate::{
    Candidate, CandidateKind, CandidateLayout, CandidateList, Cell, Language, PartOfSpeech, Sense,
    Translation,
};
pub use composition::Composition;
pub use correction::Correction;
pub use emoji::EmojiTable;
pub use engine::{
    AnnotationReport, BOOKS, Book, CloudWord, CommitEntry, Engine, FRESH_UNTIL, FilledGloss,
    Forgotten, GlossFiller, INPUT_LOG_VERSION, InputLogEntry, InputLogger, InputSource, Learner,
    LevelCount, MarkedKind, MarkedSegment, ModeKeys, NEURAL_MARGIN, NEURAL_WEIGHT, NoGlossFiller,
    NoInputLogger, NoLearner, NoPredictor, NoTranslator, NoUsageMeter, NoVocabularyTracker,
    Prediction, PredictionKind, PredictionPolicy, PredictionRequest, Predictor, QUESTION_PREFIX,
    Query, RESCORE_CONTEXT_CHARS, SurroundingText, Timings, Translator, Usage, UsageMeter,
    UsageSummary, VocabularySummary, VocabularyTracker, book_scale,
};
pub use fuzzy::FuzzyRules;
pub use history::InputHistory;
pub use parser::{ParseError, Segmentation};
pub use punctuation::Punctuation;
pub use qingjian_dictionary as dictionary;
pub use shuangpin::Scheme as ShuangpinScheme;
