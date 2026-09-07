use serde::{Deserialize, Serialize};

use crate::candidate::CandidateKind;

/// 上屏的文字从哪来。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputSource {
    /// 词库里的词（含用户词）。
    Word,

    /// 云端词。
    Cloud,

    /// 本地整句转换。
    Sentence,

    /// 英文候选。
    English,

    /// 快捷候选（日期 / 算式 / 码点）。
    Shortcut,

    /// emoji。
    Emoji,

    /// 回车原样上屏敲的字母。
    Raw,

    /// 上屏的是候选的译词（修饰键 + 数字）。
    Translation,

    /// Tab 接受的云端整句补全。
    CloudSentence,
}

impl From<CandidateKind> for InputSource {
    fn from(kind: CandidateKind) -> Self {
        match kind {
            CandidateKind::Chinese => Self::Word,
            CandidateKind::Cloud => Self::Cloud,
            CandidateKind::Sentence => Self::Sentence,
            CandidateKind::English => Self::English,
            CandidateKind::Shortcut => Self::Shortcut,
            CandidateKind::Emoji => Self::Emoji,
        }
    }
}
