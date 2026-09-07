use crate::candidate::{Candidate, CandidateKind};

/// 云端给出的一个词候选：文本加全拼音节，音节用来校验它确实对得上用户敲的拼音，也用来记成用户词。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CloudWord {
    /// 词。
    pub text: String,

    /// 全拼音节，如 `["zhang", "tao"]`。问字模式的答案不校验拼音，这里为空。
    pub syllables: Vec<String>,

    /// 显示用读音（问字模式答案的带声调拼音，如 `sēn`）。
    pub reading: Option<String>,
}

impl CloudWord {
    /// 转成云端来源的候选（译文留给 `Engine::annotate` 补）。
    pub fn into_candidate(self) -> Candidate {
        Candidate {
            text: self.text,
            kind: CandidateKind::Cloud,
            syllables: self.syllables,
            reading: self.reading,
            translation: None,
        }
    }
}
