//! 候选窗口的一行：[`Candidate`] → 渲染器的 [`Row`]（序号、候选词、annotation 片段），与 macOS 端 `candidates/row.rs` 一致。
//! GDI 画法也用同一个类型。

use qingjian_core::{Candidate, CandidateKind};
use qingjian_render::{Row, Tone};

/// `position` 是页内下标（从 0 起）。`show_code` 是 `[general] aux_code_show`：
/// 打开且候选带码时，码用方括号括起来紧跟在候选词后面（`鹤[rbm]`），不进 annotation。
/// `show_pos` 是 `[general] translation_pos`：关掉时译词保留、词性段（v. / n. / adj.）不画。
/// `english_reading_ipa` 是 `[general] show_english_pronunciation`：英文候选的 `reading` 是音标
/// （`/ˈkɑrɡoʊ/`）而不是问字模式的带调拼音，用淡色画，免得跟后面同色的译文混在一起。
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
    // 英文候选的 reading 是音标（辅助信息，排到译文之后、淡色）；其余候选的 reading 是问字模式
    // 答案的带调拼音，那是候选本身的内容，留在最前面、用译词色。
    let english_ipa = candidate
        .reading
        .as_ref()
        .filter(|_| english_reading_ipa && candidate.kind == CandidateKind::English)
        .cloned();
    if let Some(reading) = candidate.reading.as_ref().filter(|_| english_ipa.is_none()) {
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
            // 译文的音标跟在译词后面、不带括号（`hello /həˈloʊ/`）；括号是日语假名的写法
            if let Some(pronunciation) = &sense.pronunciation {
                annotation.push((format!(" {pronunciation}"), Tone::Faint));
            }
        }
    }
    // 英文候选自己的音标也排在译文之后，前面补一个空格与译文分开
    if let Some(reading) = english_ipa {
        if !annotation.is_empty() {
            annotation.push((" ".to_owned(), Tone::Separator));
        }
        annotation.push((reading, Tone::Faint));
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

    #[test]
    fn part_of_speech_follows_the_frame_switch() {
        let on = from_candidate(0, &candidate_with_gloss(), false, true, false);
        let joined: String = on
            .annotation
            .iter()
            .map(|(text, _)| text.as_str())
            .collect();
        assert!(joined.contains("v. "), "{joined}");
        assert!(joined.contains("develop"), "{joined}");

        // 关掉后词性段消失，译词与分隔结构原样保留
        let off = from_candidate(0, &candidate_with_gloss(), false, false, false);
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
            let row = from_candidate(0, &candidate, false, show_pos, false);
            let joined: String = row
                .annotation
                .iter()
                .map(|(text, _)| text.as_str())
                .collect();
            assert_eq!(joined, "develop");
        }
    }

    /// 英文候选自己的音标排在**译文之后**：`cargo n. 货物 /ˈkɑrˌɡoʊ/`，不是 `/ˈkɑrˌɡoʊ/ n. 货物`。
    #[test]
    fn english_reading_goes_after_the_translation() {
        let mut candidate = candidate_with_gloss();
        candidate.text = "cargo".to_owned();
        candidate.kind = CandidateKind::English;
        candidate.reading = Some("/ˈkɑrˌɡoʊ/".to_owned());
        let row = from_candidate(0, &candidate, false, true, true);
        let joined: String = row
            .annotation
            .iter()
            .map(|(text, _)| text.as_str())
            .collect();
        assert_eq!(joined, "v. develop /ˈkɑrˌɡoʊ/");
        // 音标用淡色
        let tone = row.annotation.last().map(|(_, tone)| *tone);
        assert_eq!(tone, Some(Tone::Faint));
    }

    /// 没有译文时音标直接跟在候选词后面，不留多余空格。
    #[test]
    fn english_reading_without_translation_has_no_leading_space() {
        let mut candidate = candidate_with_gloss();
        candidate.kind = CandidateKind::English;
        candidate.translation = None;
        candidate.reading = Some("/ˈkɑrˌɡoʊ/".to_owned());
        let row = from_candidate(0, &candidate, false, true, true);
        let joined: String = row
            .annotation
            .iter()
            .map(|(text, _)| text.as_str())
            .collect();
        assert_eq!(joined, "/ˈkɑrˌɡoʊ/");
    }

    /// 非英文候选的读音（问字模式的带调拼音）仍留在最前面。
    #[test]
    fn chinese_reading_stays_first() {
        let mut candidate = candidate_with_gloss();
        candidate.kind = CandidateKind::Chinese;
        candidate.reading = Some("kāi".to_owned());
        let row = from_candidate(0, &candidate, false, true, true);
        let joined: String = row
            .annotation
            .iter()
            .map(|(text, _)| text.as_str())
            .collect();
        assert_eq!(joined, "kāi · v. develop");
    }
}
