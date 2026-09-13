//! 候选窗口的一行：序号、候选词、annotation 片段。只是 Core 输出的展示形态，不含任何排序或查词。

use qingjian_core::Candidate;

/// annotation 片段的深浅。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// 译文。
    Gloss,

    /// 生词的译文（用户还没在候选里见过几轮，`Sense::fresh`），用强调色。
    Fresh,

    /// 词性与分隔符，最浅。
    Faint,
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
        let mut annotation = Vec::new();
        // 读音（问字模式答案的带声调拼音）放在最前
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
                let mut chars = candidate.text.chars();
                let mut preview: String = chars
                    .by_ref()
                    .take(60)
                    .map(|c| if c == '\n' || c == '\r' { '↵' } else { c })
                    .collect();
                if chars.next().is_some() {
                    preview.push('…');
                }
                preview
            } else {
                candidate.text.clone()
            },
            annotation,
            cloud: false,
        }
    }
}
