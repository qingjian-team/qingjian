//! 候选窗口的一行：序号、候选词、annotation 片段。只是 Core 输出的展示形态，不含任何排序或查词。

use qingjian_core::{Candidate, Language};

/// annotation 片段的种类，决定颜色；青简渲染器的主题还按它挑着画。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// 译文。
    Gloss,

    /// 生词的译文（用户还没在候选里见过几轮，`Sense::fresh`），用强调色。
    Fresh,

    /// 其余的淡色片段（假名注音），最浅。
    Faint,

    /// 词性，与 `Faint` 同色。
    Pos,

    /// 义项之间的分隔，与 `Faint` 同色。
    Separator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// 显示用序号文本，如 `1`。
    pub index: String,

    /// 候选词。
    pub text: String,

    /// 右侧 annotation，按顺序绘制；没有译文时为空。
    pub annotation: Vec<(String, Tone)>,

    /// 来自云联想：词前画一个小云朵，与本地候选区分。
    pub cloud: bool,
}

impl Row {
    pub fn from_candidate(position: usize, candidate: &Candidate) -> Self {
        Self::from_candidate_with_translation_pinyin(position, candidate, true)
    }

    pub fn from_candidate_with_translation_pinyin(
        position: usize,
        candidate: &Candidate,
        show_translation_pinyin: bool,
    ) -> Self {
        let mut annotation = Vec::new();
        // 读音（问字模式答案的带声调拼音）放在最前
        if let Some(reading) = &candidate.reading {
            annotation.push((reading.clone(), Tone::Gloss));
        }
        if let Some(translation) = &candidate.translation {
            for (i, sense) in translation.senses().iter().enumerate() {
                if i > 0 || !annotation.is_empty() {
                    annotation.push((" · ".to_owned(), Tone::Separator));
                }
                if let Some(pos) = sense.part_of_speech {
                    annotation.push((format!("{pos} "), Tone::Pos));
                }
                // 日文译词按汉字段注平假名（開発(かいはつ)する），假名淡色
                let tone = if sense.fresh {
                    Tone::Fresh
                } else {
                    Tone::Gloss
                };
                for segment in sense.furigana() {
                    annotation.push((segment.text, tone));
                    if let Some(reading) = segment.reading
                        && (translation.language != Language::Chinese || show_translation_pinyin)
                    {
                        annotation.push((format!("({reading})"), Tone::Faint));
                    }
                }
            }
        }
        Self {
            index: (position + 1).to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use qingjian_core::{CandidateKind, Sense, Translation};

    fn translated_candidate(language: Language, reading: &str) -> Candidate {
        Candidate {
            text: "tôi".to_owned(),
            kind: CandidateKind::VietnameseWord,
            syllables: Vec::new(),
            reading: None,
            translation: Some(Translation::new(
                language,
                vec![Sense {
                    part_of_speech: None,
                    text: "我".to_owned(),
                    reading: Some(reading.to_owned()),
                    fresh: false,
                }],
            )),
            aux_code: None,
        }
    }

    #[test]
    fn translation_pinyin_switch_only_hides_chinese_readings() {
        let chinese = translated_candidate(Language::Chinese, "wǒ");
        let shown = Row::from_candidate_with_translation_pinyin(0, &chinese, true);
        let hidden = Row::from_candidate_with_translation_pinyin(0, &chinese, false);
        assert!(shown.annotation.iter().any(|(text, _)| text == "(wǒ)"));
        assert!(!hidden.annotation.iter().any(|(text, _)| text == "(wǒ)"));

        let japanese = translated_candidate(Language::Japanese, "わたし");
        let row = Row::from_candidate_with_translation_pinyin(0, &japanese, false);
        assert!(row.annotation.iter().any(|(text, _)| text == "(わたし)"));
    }
}
