//! 候选窗口的一行：[`Candidate`] → 渲染器的 [`Row`]（序号、候选词、annotation 片段），与 macOS 端 `candidates/row.rs` 一致。
//! GDI 画法也用同一个类型。

use qingjian_core::{Candidate, CandidateKind};
use qingjian_render::{Row, Tone};

/// `position` 是页内下标（从 0 起）。`show_code` 是 `[general] aux_code_show`：
/// 打开且候选带码时，码用方括号括起来紧跟在候选词后面（`鹤[rbm]`），不进 annotation。
/// `show_pos` 是 `[general] translation_pos`：关掉时译词保留、词性段（v. / n. / adj.）不画。
pub(crate) fn from_candidate(
    position: usize,
    candidate: &Candidate,
    show_code: bool,
    show_pos: bool,
) -> Row {
    let code = candidate
        .aux_code
        .as_ref()
        .filter(|_| show_code)
        .map(|code| format!("[{code}]"));
    let mut annotation = Vec::new();
    if let Some(reading) = &candidate.reading {
        annotation.push((reading.clone(), Tone::Gloss));
    }
    if let Some(translation) = &candidate.translation {
        for (i, sense) in translation.senses().iter().enumerate() {
            if i > 0 || !annotation.is_empty() {
                annotation.push((" · ".to_owned(), Tone::Separator));
            }
            if show_pos && let Some(pos) = sense.part_of_speech {
                annotation.push((format!("{pos} "), Tone::Pos));
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
                    fresh: false,
                }],
            )),
            aux_code: None,
        }
    }

    #[test]
    fn part_of_speech_follows_the_frame_switch() {
        let on = from_candidate(0, &candidate_with_gloss(), false, true);
        let joined: String = on
            .annotation
            .iter()
            .map(|(text, _)| text.as_str())
            .collect();
        assert!(joined.contains("v. "), "{joined}");
        assert!(joined.contains("develop"), "{joined}");

        // 关掉后词性段消失，译词与分隔结构原样保留
        let off = from_candidate(0, &candidate_with_gloss(), false, false);
        let joined: String = off
            .annotation
            .iter()
            .map(|(text, _)| text.as_str())
            .collect();
        assert!(!joined.contains("v. "), "{joined}");
        assert!(joined.contains("develop"), "{joined}");
        assert_eq!(on.annotation.len() - 1, off.annotation.len());
    }

    #[test]
    fn a_gloss_without_part_of_speech_is_unchanged_by_the_switch() {
        let mut candidate = candidate_with_gloss();
        candidate.translation.as_mut().unwrap().senses_mut()[0].part_of_speech = None;
        for show_pos in [true, false] {
            let row = from_candidate(0, &candidate, false, show_pos);
            let joined: String = row
                .annotation
                .iter()
                .map(|(text, _)| text.as_str())
                .collect();
            assert_eq!(joined, "develop");
        }
    }
}
