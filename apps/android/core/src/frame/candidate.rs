//! 帧里的一个候选。
use qingjian_core::{Candidate, CandidateKind};
use serde::Serialize;

use super::sense::FrameSense;

/// 候选条画一个候选要的东西；Core 的 `Candidate` 不直接过 JNI，免得 Kotlin 侧跟着 serde 形状走。
#[derive(Debug, Serialize)]
pub struct FrameCandidate {
    pub text: String,

    /// 学习语言下的译文，最多两条；下标就是 `Session::select_translation` 的 `sense`。
    pub senses: Vec<FrameSense>,

    /// 长按能不能删（Core 的 `CandidateKind::forgettable`）。
    pub forgettable: bool,

    /// 来自云联想（云端词或整句补全）：候选条上画小云朵、用云端色。
    pub cloud: bool,
}

impl FrameCandidate {
    /// 没有译文的候选（补全）。
    pub fn plain(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            senses: Vec::new(),
            forgettable: false,
            cloud: false,
        }
    }
}

impl From<&Candidate> for FrameCandidate {
    fn from(candidate: &Candidate) -> Self {
        Self {
            text: candidate.text.clone(),
            senses: candidate
                .translation
                .iter()
                .flat_map(|translation| translation.senses())
                .map(FrameSense::from)
                .collect(),
            forgettable: candidate.kind.forgettable(),
            cloud: candidate.kind == CandidateKind::Cloud,
        }
    }
}
