//! 候选窗口的一行：序号、候选词、annotation 片段。只是 Core 输出的展示形态，不含任何排序或查词。

use qingjian_core::Candidate;

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

    /// 紧跟在候选词后面的辅码（`[rbm]`），开关关着或词没有码时为 `None`。
    pub code: Option<String>,

    /// 右侧 annotation，按顺序绘制；没有译文时为空。
    pub annotation: Vec<(String, Tone)>,

    /// 来自云联想：词前画一个小云朵，与本地候选区分。
    pub cloud: bool,
}

impl Row {
    /// `show_pos` 是 `[general] translation_pos`：关掉时译词保留、词性段（v. / n. / adj.）不画；
    /// `show_code` 是 `[general] aux_code_show`。
    pub fn from_candidate(
        position: usize,
        candidate: &Candidate,
        show_pos: bool,
        show_code: bool,
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
                if show_pos && let Some(pos) = sense.part_of_speech {
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
                    if let Some(reading) = segment.reading {
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
            code: candidate
                .aux_code
                .as_ref()
                .filter(|_| show_code)
                .map(|code| format!("[{code}]")),
            annotation,
            cloud: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use qingjian_core::{CandidateKind, Language, PartOfSpeech, Sense, Translation};

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

    fn joined(row: &Row) -> String {
        row.annotation
            .iter()
            .map(|(text, _)| text.as_str())
            .collect()
    }

    #[test]
    fn part_of_speech_follows_the_switch() {
        assert_eq!(
            joined(&Row::from_candidate(
                0,
                &candidate_with_gloss(),
                true,
                false
            )),
            "v. develop"
        );
        assert_eq!(
            joined(&Row::from_candidate(
                0,
                &candidate_with_gloss(),
                false,
                false
            )),
            "develop"
        );
    }
}
