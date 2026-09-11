//! 候选窗口的一行：序号、候选词、右侧 annotation 片段（读音 / 词性 / 译文）。
//!
//! 只是 Server 回来的 [`Candidate`] 的展示形态，不含任何排序或查词——排序 / 查词都在 Core（Server 进程）。
//! 逻辑与 macOS 端 `candidates/row.rs` 一致：读音在前，译文按义项拼，日文按汉字段注平假名，生词用强调色。

use qingjian_core::Candidate;

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

    /// 来自云联想：词前画一个小云朵，与本地候选区分。
    pub cloud: bool,
}

impl Row {
    /// 从一个候选造一行。`position` 是页内下标（从 0 起），序号显示为 `position + 1`。
    pub(crate) fn from_candidate(position: usize, candidate: &Candidate) -> Self {
        let mut annotation = Vec::new();
        // 读音（问字模式答案的带声调拼音）放在最前。
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
                // 日文译词按汉字段注平假名（開発(かいはつ)する），假名淡色。
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
            // 云端候选的区分下一步接（协议里补 kind 判定）；先都当本地词。
            cloud: false,
        }
    }
}
