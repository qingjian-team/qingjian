//! 帧 → IBus 面板要的三样东西。
//!
//! IBus 的面板只认 preedit、候选表、辅助行（auxiliary text）这三样，没有自绘那么自由：
//! 候选旁的译词、整句补全、删候选提示都得挤进这三样里。怎么挤在这里定，
//! 换成方案 A 自绘时整个文件作废，Router 与 Core 一行不动。

use qingjian_platform::protocol::{Frame, PreeditKind};

use super::variant::{Segment, Style};

/// 一帧折成面板要的形态。
pub struct View {
    /// preedit 的分段（带样式）。
    pub preedit: Vec<Segment>,

    /// 光标在 preedit 里的字符位置。
    pub cursor: u32,

    /// 当前页的候选文本。
    pub candidates: Vec<String>,

    /// 高亮在当前页里的下标。
    pub cursor_index: u32,

    /// 辅助行：页码、整句补全、删候选提示挤在这里。没有就是 `None`。
    pub auxiliary: Option<String>,
}

impl View {
    /// 按一帧折出来。
    pub fn from_frame(frame: &Frame) -> Self {
        Self {
            preedit: frame
                .preedit
                .iter()
                .map(|segment| Segment {
                    text: segment.text.clone(),
                    style: match segment.kind {
                        PreeditKind::Corrected => Style::Corrected,
                        _ => Style::Underline,
                    },
                })
                .collect(),
            cursor: frame.cursor as u32,
            candidates: candidate_labels(frame),
            cursor_index: if frame.highlight == usize::MAX {
                0
            } else {
                frame.highlight as u32
            },
            auxiliary: auxiliary(frame),
        }
    }

    /// 这一帧要不要显示候选表。
    pub fn has_candidates(&self) -> bool {
        !self.candidates.is_empty()
    }
}

/// 候选文本：**译词跟在候选后面**，中间两个空格。
///
/// 自绘时译词是右侧一列、更小更浅（见 `docs/design/candidate-ui.md` 的视觉层级），
/// 而 IBus 的候选只能是一条 `IBusText`，分不了列也分不了字号。
/// 两害相权：译词是青简的卖点，宁可挤在同一行也不能不显示。
fn candidate_labels(frame: &Frame) -> Vec<String> {
    frame
        .candidates
        .items
        .iter()
        .map(|candidate| {
            let mut label = candidate.text.clone();
            if let Some(reading) = &candidate.reading {
                label.push_str("  ");
                label.push_str(reading);
            }
            if let Some(translation) = &candidate.translation
                && let Some(sense) = translation.senses().first()
            {
                label.push_str("  ");
                if let Some(pos) = sense.part_of_speech {
                    label.push_str(&format!("{pos} "));
                }
                label.push_str(&sense.text);
            }
            label
        })
        .collect()
}

/// 辅助行：删候选提示优先（它是对刚才那次操作的回答），其次整句补全，最后页码。
fn auxiliary(frame: &Frame) -> Option<String> {
    if let Some(notice) = &frame.notice {
        return Some(notice.clone());
    }
    let mut parts = Vec::new();
    if let Some(sentence) = &frame.sentence {
        parts.push(format!("⇥ {sentence}"));
    }
    if frame.page_count > 1 {
        parts.push(format!("{}/{}", frame.page + 1, frame.page_count));
    }
    (!parts.is_empty()).then(|| parts.join("   "))
}

#[cfg(test)]
mod tests {
    use qingjian_core::{Candidate, CandidateKind, CandidateList};
    use qingjian_platform::protocol::PreeditSegment;

    use super::*;

    fn frame_with(candidates: Vec<Candidate>) -> Frame {
        Frame {
            preedit: vec![PreeditSegment {
                text: "nihao".to_owned(),
                kind: PreeditKind::Typed,
            }],
            cursor: 5,
            candidates: CandidateList { items: candidates },
            highlight: 0,
            page: 0,
            page_count: 1,
            ..Frame::default()
        }
    }

    fn candidate(text: &str) -> Candidate {
        Candidate {
            text: text.to_owned(),
            kind: CandidateKind::Chinese,
            syllables: Vec::new(),
            reading: None,
            translation: None,
            aux_code: None,
        }
    }

    #[test]
    fn empty_frame_has_nothing_to_show() {
        let view = View::from_frame(&Frame::default());
        assert!(view.preedit.is_empty());
        assert!(!view.has_candidates());
        assert!(view.auxiliary.is_none());
    }

    #[test]
    fn candidates_come_through_in_order() {
        let view = View::from_frame(&frame_with(vec![candidate("你好"), candidate("你号")]));
        assert_eq!(view.candidates, vec!["你好", "你号"]);
        assert!(view.has_candidates());
    }

    /// 高亮是 `usize::MAX`（没有候选可高亮）时不能直接转成 u32 溢出。
    #[test]
    fn absent_highlight_falls_back_to_zero() {
        let frame = Frame {
            highlight: usize::MAX,
            ..frame_with(Vec::new())
        };
        assert_eq!(View::from_frame(&frame).cursor_index, 0);
    }

    #[test]
    fn page_number_goes_to_the_auxiliary_line() {
        let frame = Frame {
            page: 1,
            page_count: 3,
            ..frame_with(vec![candidate("你好")])
        };
        assert_eq!(View::from_frame(&frame).auxiliary.as_deref(), Some("2/3"));
    }

    /// 只有一页时不写页码，免得辅助行一直占着。
    #[test]
    fn single_page_has_no_page_number() {
        let view = View::from_frame(&frame_with(vec![candidate("你好")]));
        assert!(view.auxiliary.is_none());
    }

    /// 删候选的提示压过页码与整句补全：它是对刚才那次按键的回答。
    #[test]
    fn notice_wins_the_auxiliary_line() {
        let frame = Frame {
            notice: Some("已删除用户词「你号」".to_owned()),
            sentence: Some("你好吗".to_owned()),
            page: 1,
            page_count: 3,
            ..frame_with(vec![candidate("你好")])
        };
        assert_eq!(
            View::from_frame(&frame).auxiliary.as_deref(),
            Some("已删除用户词「你号」")
        );
    }

    #[test]
    fn sentence_completion_and_page_share_the_line() {
        let frame = Frame {
            sentence: Some("你好吗".to_owned()),
            page: 0,
            page_count: 2,
            ..frame_with(vec![candidate("你好")])
        };
        assert_eq!(
            View::from_frame(&frame).auxiliary.as_deref(),
            Some("⇥ 你好吗   1/2")
        );
    }

    /// 纠错段换成另一种样式（IBus 没有删除线）。
    #[test]
    fn corrected_preedit_gets_its_own_style() {
        let frame = Frame {
            preedit: vec![PreeditSegment {
                text: "nihao".to_owned(),
                kind: PreeditKind::Corrected,
            }],
            ..Frame::default()
        };
        let view = View::from_frame(&frame);
        assert!(matches!(view.preedit[0].style, Style::Corrected));
    }
}
