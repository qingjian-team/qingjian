//! 候选窗口的一行：[`Candidate`] → [`Row`]（序号、候选词、注记片段），规则与 Server 的 GDI 画法一致。
//!
//! 这里不借用 Server 的类型：TSF DLL 被加载进每个应用进程，多链一个 crate 就多一份体积，
//! 这一层自己算一遍（规则与 `apps/windows/server/src/ui/candidates/row.rs` 相同）。

use qingjian_core::{Candidate, CandidateKind};

/// 注记片段的配色语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tone {
    /// 译文。
    Gloss,

    /// 生词译文。
    Fresh,

    /// 音标等辅助信息，比译文更淡。
    Faint,

    /// 词性、分隔符。
    Pos,

    /// 辅码。
    Code,
}

/// 一段注记：文字、配色，以及它属于第几条译词（点译文上屏那一条时用，没有就是 `None`）。
#[derive(Debug, Clone)]
pub(crate) struct Seg {
    pub(crate) text: String,

    pub(crate) tone: Tone,

    pub(crate) sense: Option<usize>,
}

/// 一行候选。
#[derive(Debug, Clone, Default)]
pub(crate) struct Row {
    /// 序号；矩阵里只标高亮所在那一行，其余为空。
    pub(crate) index: String,

    pub(crate) text: String,

    /// 辅码，`[general] aux_code_show` 打开且候选带码时是 `[rbm]` 这种形状。
    pub(crate) code: Option<String>,

    /// 候选词右侧的注记（读音、词性 + 译文、音标）。
    pub(crate) annotation: Vec<Seg>,

    /// 云端候选：前面画一朵小云。
    pub(crate) cloud: bool,
}

/// `position` 是页内下标（从 0 起）。`show_code` 是 `[general] aux_code_show`；
/// `show_pos` 是 `[general] translation_pos`；`english_reading_ipa` 是 `[general] show_english_pronunciation`。
/// 规则与 Server 侧 [`from_candidate`](../../../../server/src/ui/candidates/row.rs) 一致。
pub(crate) fn from_candidate(
    position: usize,
    candidate: &Candidate,
    show_code: bool,
    show_pos: bool,
    english_reading_ipa: bool,
) -> Row {
    let code = candidate
        .aux_code
        .as_ref()
        .filter(|_| show_code)
        .map(|code| format!("[{code}]"));
    let mut annotation = Vec::new();
    let english_ipa = candidate
        .reading
        .as_ref()
        .filter(|_| english_reading_ipa && candidate.kind == CandidateKind::English)
        .cloned();
    if let Some(reading) = candidate.reading.as_ref().filter(|_| english_ipa.is_none()) {
        annotation.push(Seg {
            text: reading.clone(),
            tone: Tone::Gloss,
            sense: None,
        });
    }
    if let Some(translation) = &candidate.translation {
        for (i, sense) in translation.senses().iter().enumerate() {
            if i > 0 || !annotation.is_empty() {
                annotation.push(Seg {
                    text: " · ".to_owned(),
                    tone: Tone::Pos,
                    sense: None,
                });
            }
            if show_pos && let Some(pos) = sense.part_of_speech {
                annotation.push(Seg {
                    text: format!("{pos} "),
                    tone: Tone::Pos,
                    sense: Some(i),
                });
            }
            let tone = if sense.fresh {
                Tone::Fresh
            } else {
                Tone::Gloss
            };
            for segment in sense.furigana() {
                annotation.push(Seg {
                    text: segment.text,
                    tone,
                    sense: Some(i),
                });
                if let Some(reading) = segment.reading {
                    annotation.push(Seg {
                        text: format!("({reading})"),
                        tone: Tone::Faint,
                        sense: Some(i),
                    });
                }
            }
            if let Some(pronunciation) = &sense.pronunciation {
                annotation.push(Seg {
                    text: format!(" {pronunciation}"),
                    tone: Tone::Faint,
                    sense: Some(i),
                });
            }
        }
    }
    if let Some(reading) = english_ipa {
        if !annotation.is_empty() {
            annotation.push(Seg {
                text: " ".to_owned(),
                tone: Tone::Pos,
                sense: None,
            });
        }
        annotation.push(Seg {
            text: reading,
            tone: Tone::Faint,
            sense: None,
        });
    }
    Row {
        index: (position + 1).to_string(),
        text: candidate.text.clone(),
        code,
        annotation,
        cloud: candidate.kind == CandidateKind::Cloud,
    }
}

#[cfg(test)]
mod tests {
    use qingjian_core::{Language, PartOfSpeech, Sense, Translation};

    use super::*;

    fn candidate_with_gloss() -> Candidate {
        Candidate {
            text: "开发".to_owned(),
            kind: CandidateKind::Chinese,
            syllables: vec!["kai".to_owned(), "fa".to_owned()],
            reading: None,
            translation: Some(Translation::new(
                Language::English,
                vec![Sense {
                    part_of_speech: Some(PartOfSpeech::Verb),
                    text: "develop".to_owned(),
                    reading: None,
                    pronunciation: None,
                    fresh: false,
                }],
            )),
            aux_code: None,
        }
    }

    fn joined(row: &Row) -> String {
        row.annotation.iter().map(|seg| seg.text.as_str()).collect()
    }

    /// 词性跟着帧里的开关走：关掉后只剩译词。
    #[test]
    fn part_of_speech_follows_the_frame_switch() {
        let candidate = candidate_with_gloss();
        let on = joined(&from_candidate(0, &candidate, false, true, false));
        let off = joined(&from_candidate(0, &candidate, false, false, false));
        assert!(on.contains("v. "), "{on}");
        assert!(off.contains("develop"), "{off}");
        assert!(!off.contains("v. "), "{off}");
    }

    /// 译词片段带上它属于第几条译词，点译文上屏靠它。
    #[test]
    fn gloss_segments_carry_their_sense() {
        let row = from_candidate(0, &candidate_with_gloss(), false, true, false);
        let sense = row
            .annotation
            .iter()
            .find(|seg| seg.text.contains("develop"))
            .and_then(|seg| seg.sense);
        assert_eq!(sense, Some(0));
        // 分隔符不属于任何一条译词
        let separator = row
            .annotation
            .iter()
            .find(|seg| seg.text.contains('·'))
            .and_then(|seg| seg.sense);
        assert_eq!(separator, None);
    }

    /// 英文候选的音标排在译文之后、用淡色。
    #[test]
    fn english_reading_goes_after_the_translation() {
        let mut candidate = candidate_with_gloss();
        candidate.text = "cargo".to_owned();
        candidate.kind = CandidateKind::English;
        candidate.reading = Some("/ˈkɑrˌɡoʊ/".to_owned());
        let row = from_candidate(0, &candidate, false, true, true);
        assert_eq!(joined(&row), "v. develop /ˈkɑrˌɡoʊ/");
        assert_eq!(row.annotation.last().map(|seg| seg.tone), Some(Tone::Faint));
    }

    /// 非英文候选的读音（问字模式的带调拼音）仍留在最前面。
    #[test]
    fn chinese_reading_stays_first() {
        let mut candidate = candidate_with_gloss();
        candidate.reading = Some("kāi".to_owned());
        let row = from_candidate(0, &candidate, false, true, true);
        assert_eq!(joined(&row), "kāi · v. develop");
    }
}
