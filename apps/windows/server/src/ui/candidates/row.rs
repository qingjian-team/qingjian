//! 候选窗口的一行：[`Candidate`] 的展示形态，与 macOS 端 `candidates/row.rs` 一致。

use qingjian_core::{Candidate, CandidateKind};

/// annotation 片段的深浅。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tone {
    /// 译文。
    Gloss,

    /// 生词的译文（`Sense::fresh`），用强调色。
    Fresh,

    /// 词性、假名注音与分隔符，最浅。
    Faint,
}

/// 候选窗口的一行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Row {
    /// 显示用序号文本，如 `1`。
    pub index: String,

    /// 候选词本体。
    pub text: String,

    /// 右侧 annotation，按顺序绘制；没有译文时为空。
    pub annotation: Vec<(String, Tone)>,

    /// 来自云联想：词前画一个小云朵。
    pub cloud: bool,
}

impl Row {
    /// `position` 是页内下标（从 0 起）。
    pub(crate) fn from_candidate(position: usize, candidate: &Candidate) -> Self {
        let mut annotation = Vec::new();
        if let Some(reading) = &candidate.reading {
            annotation.push((reading.clone(), Tone::Gloss));
        }
        if let Some(translation) = &candidate.translation {
            for (i, sense) in translation.senses().iter().enumerate() {
                if i > 0 || !annotation.is_empty() {
                    annotation.push((" · ".to_owned(), Tone::Faint));
                }
                if let Some(pos) = sense.part_of_speech {
                    annotation.push((format!("{pos} "), Tone::Faint));
                }
                let tone = if sense.fresh {
                    Tone::Fresh
                } else {
                    Tone::Gloss
                };
                for segment in sense.furigana() {
                    annotation.push((segment.text, tone));
                    if let Some(reading) = segment.reading {
                        annotation.push((format!("({reading})"), Tone::Faint));
                    }
                }
            }
        }
        Self {
            index: (position + 1).to_string(),
            text: candidate.text.clone(),
            annotation,
            cloud: candidate.kind == CandidateKind::Cloud,
        }
    }
}
