//! 候选词数据模型。
//!
//! 翻译是候选词的 annotation：可选、单语言、最多 [`Translation::MAX_SENSES`] 条释义。
//! 不要把它扩展成多语言并列的结构，那会破坏「一次只学一种语言」的产品原则。

mod furigana;
mod kind;
mod language;
mod layout;
mod list;
mod part_of_speech;
mod sense;
mod translation;

use serde::{Deserialize, Serialize};

pub use furigana::{FuriganaSegment, furigana};
pub use kind::CandidateKind;
pub use language::{Language, UnknownLanguage};
pub use layout::{CandidateLayout, Cell};
pub use list::CandidateList;
pub use part_of_speech::{PartOfSpeech, UnknownPartOfSpeech};
pub use sense::Sense;
pub use translation::Translation;

/// 一个可上屏的候选。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    /// 上屏文本。
    pub text: String,

    /// 来源类型。
    pub kind: CandidateKind,

    /// 该候选对应的拼音音节，供平台层高亮已匹配部分。
    pub syllables: Vec<String>,

    /// 读音（如日语假名），中文候选暂不使用。
    pub reading: Option<String>,

    /// 学习语言下的译文；查不到或尚未就绪时为 `None`。
    pub translation: Option<Translation>,
}
