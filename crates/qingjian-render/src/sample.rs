//! 样例帧：主题预览图（[`crate::preview`]）、离线预览（`examples/preview.rs`）与快照测试共用。
//! 与真机上敲同样拼音看到的候选窗对照，内容要和引擎当时给的一致（人工从截图抄）。

use crate::{Frame, Mode, Preedit, Row, Tone};

/// 真机敲「nihao」看到的第一页（2026-09-13 从截图抄），拼音行带光标、译文、生词橙色、页码。
pub fn nihao() -> Frame {
    Frame {
        preedit: Some(Preedit::plain("ni'hao", 6)),
        rows: vec![
            annotated(
                0,
                "你好",
                &[
                    ("int. ", Tone::Pos),
                    ("hello", Tone::Gloss),
                    (" · ", Tone::Separator),
                    ("int. ", Tone::Pos),
                    ("hi", Tone::Gloss),
                ],
                false,
            ),
            annotated(1, "👋", &[("你好", Tone::Gloss)], false),
            annotated(2, "你好好", &[], false),
            annotated(
                3,
                "你好像",
                &[("phr. ", Tone::Pos), ("you seem", Tone::Fresh)],
                false,
            ),
            annotated(4, "你好久", &[], false),
            annotated(5, "你好看", &[], false),
            annotated(6, "你哈", &[], false),
            annotated(
                7,
                "你换",
                &[
                    ("phr. ", Tone::Pos),
                    ("you change", Tone::Fresh),
                    (" · ", Tone::Separator),
                    ("phr. ", Tone::Pos),
                    ("you swap", Tone::Fresh),
                ],
                false,
            ),
            annotated(
                8,
                "你会",
                &[("phr. ", Tone::Pos), ("you will", Tone::Fresh)],
                false,
            ),
        ],
        highlighted: Some(0),
        footer: Some("1/6".to_owned()),
        sentence: None,
        status: None,
        mode: Mode::default(),
        columns: 0,
        column_ems: Vec::new(),
    }
}

/// 横排真机截图那一次云端整句到了：拼音行右侧带云朵的整句补全。
pub fn nihao_with_sentence() -> Frame {
    let mut frame = nihao();
    frame.sentence = Some("你好，很高兴认识你！".to_owned());
    frame
}

/// 一行候选：序号从 0 数，`annotation` 是译文各段与色调。
pub fn annotated(index: usize, text: &str, annotation: &[(&str, Tone)], cloud: bool) -> Row {
    Row {
        index: (index + 1).to_string(),
        text: text.to_owned(),
        annotation: annotation
            .iter()
            .map(|(s, tone)| ((*s).to_owned(), *tone))
            .collect(),
        cloud,
        code: None,
    }
}
