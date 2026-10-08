//! 候选窗口的一行：序号、候选词、annotation 片段。只是 Core 输出的展示形态，不含任何排序或查词。

use qingjian_core::{Candidate, CandidateKind, Language};

/// annotation 片段的显示样式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// 译文。
    Gloss,

    /// 生词的译文（用户还没在候选里见过几轮，`Sense::fresh`），用强调色。
    Fresh,

    /// 词性有独立字号，颜色与淡色提示相同。
    PartOfSpeech,

    /// 读音或 emoji 原词提示，颜色沿用普通词，字体独立按提示语言选择。
    Reading { chinese: bool },

    /// 分隔符与注音，最浅。
    Faint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// 显示用序号文本，如 `1`。
    pub index: String,

    /// 候选词。
    pub text: String,

    /// 英文候选主体使用外语字体。
    pub foreign_text: bool,

    /// 中文释义使用中文字体，与主体类型无关；读音/提示单独标记。
    pub chinese_annotation: bool,

    /// 右侧 annotation，按顺序绘制；没有译文时为空。
    pub annotation: Vec<(String, Tone)>,

    /// 来自云联想：词前画一个小云朵，与本地候选区分。
    pub cloud: bool,
}

impl Row {
    pub fn from_candidate(position: usize, candidate: &Candidate) -> Self {
        let mut annotation = Vec::new();
        // 读音（问字模式答案的带声调拼音）放在最前
        if let Some(reading) = &candidate.reading {
            annotation.push((
                reading.clone(),
                Tone::Reading {
                    chinese: candidate.kind == CandidateKind::Emoji && chinese_label(reading),
                },
            ));
        }
        if let Some(translation) = &candidate.translation {
            for (i, sense) in translation.senses().iter().enumerate() {
                if i > 0 || !annotation.is_empty() {
                    annotation.push((" · ".to_owned(), Tone::Faint));
                }
                if let Some(pos) = sense.part_of_speech {
                    annotation.push((format!("{pos} "), Tone::PartOfSpeech));
                }
                // 日文译词按汉字段注平假名（開発(かいはつ)する），假名淡色
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
            foreign_text: candidate.kind == CandidateKind::English,
            chinese_annotation: candidate
                .translation
                .as_ref()
                .is_some_and(|translation| translation.language == Language::Chinese),
            text: if matches!(candidate.kind, qingjian_core::CandidateKind::Custom(_)) {
                qingjian_core::CustomPhrase::preview(&candidate.text, 60)
            } else {
                candidate.text.clone()
            },
            annotation,
            cloud: false,
        }
    }
}

/// emoji 的原词提示复用 reading 字段，没有语言标签；只为选显示字体检查汉字，不改提示文本。
fn chinese_label(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(c as u32,
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x323AF | 0x3005 | 0x3007)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use qingjian_core::{CandidateKind, Language, PartOfSpeech, Sense, Translation};

    #[test]
    fn english_candidates_reverse_fonts_and_pos_stays_distinct_from_readings() {
        let mut candidate = Candidate {
            text: "learn".into(),
            kind: CandidateKind::English,
            syllables: vec![],
            reading: None,
            aux_code: None,
            translation: Some(Translation::new(
                Language::Chinese,
                vec![Sense {
                    text: "学习".into(),
                    part_of_speech: Some(PartOfSpeech::Verb),
                    reading: None,
                    fresh: false,
                }],
            )),
        };
        let row = Row::from_candidate(0, &candidate);
        assert!(row.foreign_text);
        assert!(row.chinese_annotation);
        assert_eq!(
            row.annotation,
            [
                ("v. ".into(), Tone::PartOfSpeech),
                ("学习".into(), Tone::Gloss)
            ]
        );
        candidate.kind = CandidateKind::Chinese;
        candidate.translation.as_mut().unwrap().language = Language::English;
        candidate.reading = Some("xué xí".into());
        let row = Row::from_candidate(0, &candidate);
        assert!(!row.foreign_text);
        assert!(!row.chinese_annotation);
        assert_eq!(row.annotation[0].1, Tone::Reading { chinese: false });
        assert_eq!(row.annotation[1].1, Tone::Faint);
        assert_eq!(row.annotation[2].1, Tone::PartOfSpeech);
    }

    #[test]
    fn emoji_reading_uses_chinese_typography_only_for_chinese_labels() {
        // 与 Engine::insert_emoji 一致：提示放在 reading，translation 为空。
        let mut candidate = Candidate {
            text: "🔶".into(),
            kind: CandidateKind::Emoji,
            syllables: vec![],
            reading: None,
            translation: None,
            aux_code: None,
        };
        for (label, chinese) in [
            ("大", true),
            ("塔", true),
            ("𠮷", true),
            ("smile", false),
            ("xué xí", false),
        ] {
            candidate.reading = Some(label.into());
            let row = Row::from_candidate(0, &candidate);
            assert!(!row.foreign_text, "emoji 本体字号不应随提示语言改变");
            assert!(!row.chinese_annotation);
            assert_eq!(row.annotation, [(label.into(), Tone::Reading { chinese })]);
        }
        // 中文原词提示与外语译词同时存在时，不能把后者也改成中文字体。
        candidate.reading = Some("大".into());
        candidate.translation = Some(Translation::new(
            Language::English,
            vec![Sense {
                text: "big".into(),
                part_of_speech: Some(PartOfSpeech::Adjective),
                reading: None,
                fresh: true,
            }],
        ));
        let row = Row::from_candidate(0, &candidate);
        assert!(!row.chinese_annotation);
        assert_eq!(row.annotation[0].1, Tone::Reading { chinese: true });
        assert_eq!(row.annotation.last().unwrap(), &("big".into(), Tone::Fresh));
    }
}
