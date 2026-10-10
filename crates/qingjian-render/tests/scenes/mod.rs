//! 样例帧：离线预览（`examples/preview.rs`）与快照测试（`tests/snapshots.rs`）共用。
//! 前几个与真机上敲同样拼音看到的候选窗对照，内容要和引擎当时给的一致（人工从截图抄）。

#![allow(dead_code)]

pub use qingjian_render::sample::{annotated, nihao, nihao_with_sentence};
use qingjian_render::{
    Frame, Layout, Mode, Preedit, PreeditSegment, PreeditStyle, Row, StatusCell, Tone,
};

/// 候选窗样例：名字、帧、排布。
pub fn candidate_scenes() -> Vec<(&'static str, Frame, Layout)> {
    vec![
        ("nihao-vertical", nihao(), Layout::Vertical),
        (
            "nihao-horizontal",
            nihao_with_sentence(),
            Layout::Horizontal,
        ),
        ("cloud-vertical", cloud(), Layout::Vertical),
        ("cloud-horizontal", cloud(), Layout::Horizontal),
        ("corrected-vertical", corrected_japanese(), Layout::Vertical),
        (
            "corrected-horizontal",
            corrected_japanese(),
            Layout::Horizontal,
        ),
        ("probe", probe(), Layout::Vertical),
        ("preedit-only", preedit_only(), Layout::Vertical),
        ("short-vertical", short(), Layout::Vertical),
        ("aux-code-vertical", aux_code(), Layout::Vertical),
        ("aux-code-horizontal", aux_code(), Layout::Horizontal),
    ]
}

/// Windows 悬浮状态条的三格。
pub fn status_cells() -> Vec<StatusCell> {
    vec![
        StatusCell::text("中 · 小鹤", true),
        StatusCell::text("，。", true),
        StatusCell::Gear,
    ]
}

/// 带云联想的一页：整句补全与云端词各带云朵。
pub fn cloud() -> Frame {
    let mut frame = nihao();
    frame.rows.truncate(3);
    frame
        .rows
        .push(annotated(3, "你好吗", &[("hello?", Tone::Gloss)], true));
    frame.sentence = Some("你好，世界".to_owned());
    frame.footer = Some("1/3".to_owned());
    frame
}

/// 纠错后的拼音行（删除线 + 淡色剩余）加日文译词（汉字注假名）。
pub fn corrected_japanese() -> Frame {
    Frame {
        preedit: Some(Preedit {
            segments: vec![
                PreeditSegment {
                    text: "kai".to_owned(),
                    style: PreeditStyle::Typed,
                },
                PreeditSegment {
                    text: "fs".to_owned(),
                    style: PreeditStyle::Struck,
                },
                PreeditSegment {
                    text: "'fa".to_owned(),
                    style: PreeditStyle::Rest,
                },
            ],
            cursor: 5,
        }),
        rows: vec![
            annotated(
                0,
                "开发",
                &[
                    ("v. ", Tone::Pos),
                    ("開発", Tone::Gloss),
                    ("(かいはつ)", Tone::Faint),
                    ("する", Tone::Gloss),
                ],
                false,
            ),
            annotated(
                1,
                "开",
                &[
                    ("v. ", Tone::Pos),
                    ("開", Tone::Fresh),
                    ("(ひら)", Tone::Faint),
                    ("く", Tone::Fresh),
                ],
                false,
            ),
        ],
        highlighted: Some(1),
        footer: None,
        sentence: None,
        status: Some("已删除「开放」".to_owned()),
        mode: Mode::default(),
        columns: 0,
        column_ems: Vec::new(),
    }
}

/// 四条验收用的一行：汉字（zh 字形）、英文、彩色 emoji、日文假名与汉字。
pub fn probe() -> Frame {
    Frame {
        preedit: None,
        rows: vec![Row::plain(0, "青简 hello 🙂 日本語 骨直曜")],
        highlighted: None,
        footer: None,
        sentence: None,
        status: None,
        mode: Mode::default(),
        columns: 0,
        column_ems: Vec::new(),
    }
}

/// 辅码态：拼音行末尾是触发键与带下划线的码段，候选词后面跟着命中的码。
pub fn aux_code() -> Frame {
    let mut rows = vec![
        annotated(
            0,
            "开发",
            &[("v. ", Tone::Pos), ("develop", Tone::Gloss)],
            false,
        ),
        annotated(
            1,
            "开饭",
            &[("v. ", Tone::Pos), ("serve meal", Tone::Gloss)],
            false,
        ),
        annotated(2, "咖啡", &[], false),
    ];
    for (row, code) in rows.iter_mut().zip(["[kf]", "[kfu]", "[kfe]"]) {
        row.code = Some(code.to_owned());
    }
    Frame {
        preedit: Some(Preedit {
            segments: vec![
                PreeditSegment {
                    text: "kai'fa;".into(),
                    style: PreeditStyle::Typed,
                },
                PreeditSegment {
                    text: "kf".into(),
                    style: PreeditStyle::AuxCode,
                },
            ],
            cursor: 9,
        }),
        rows,
        highlighted: Some(0),
        footer: Some("1/1".into()),
        sentence: None,
        status: None,
        mode: Mode::default(),
        columns: 0,
        column_ems: Vec::new(),
    }
}

/// 组句中还没有候选：只有拼音行。
pub fn preedit_only() -> Frame {
    Frame {
        preedit: Some(Preedit::plain("zhong'wen'shu'ru", 8)),
        ..Frame::default()
    }
}

/// 候选都很短、没有译文、不高亮：竖排窗口要撑到最小宽度。
pub fn short() -> Frame {
    Frame {
        preedit: Some(Preedit::plain("a", 1)),
        rows: vec![
            Row::plain(0, "啊"),
            Row::plain(1, "阿"),
            Row::plain(2, "吖"),
        ],
        highlighted: None,
        footer: None,
        sentence: None,
        status: None,
        mode: Mode::default(),
        columns: 0,
        column_ems: Vec::new(),
    }
}
