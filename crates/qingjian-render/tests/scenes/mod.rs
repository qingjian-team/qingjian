//! 样例帧：离线预览（`examples/preview.rs`）与快照测试（`tests/snapshots.rs`）共用。
//! 前几个与真机上敲同样拼音看到的候选窗对照，内容要和引擎当时给的一致（人工从截图抄）。

#![allow(dead_code)]

pub use qingjian_render::sample::{annotated, nihao, nihao_with_sentence};
use qingjian_render::{
    Frame, Keyboard, Keycap, KeycapState, Layout, Mode, Preedit, PreeditSegment, PreeditStyle,
    Root, Row, StatusCell, Tone,
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

/// 键、翘舌声母、韵母。
type XiaoheKey = (char, Option<&'static str>, &'static [&'static str]);

/// 小鹤双拼的键位：键、翘舌声母、韵母（从 `qingjian_core::shuangpin::KeyHint` 抄）。
const XIAOHE: [&[XiaoheKey]; 3] = [
    &[
        ('q', None, &["iu"]),
        ('w', None, &["ei"]),
        ('e', None, &["e"]),
        ('r', None, &["uan"]),
        ('t', None, &["üe", "ue"]),
        ('y', None, &["un"]),
        ('u', Some("sh"), &["u"]),
        ('i', Some("ch"), &["i"]),
        ('o', None, &["uo", "o"]),
        ('p', None, &["ie"]),
    ],
    &[
        ('a', None, &["a"]),
        ('s', None, &["iong", "ong"]),
        ('d', None, &["ai"]),
        ('f', None, &["en"]),
        ('g', None, &["eng"]),
        ('h', None, &["ang"]),
        ('j', None, &["an"]),
        ('k', None, &["ing", "uai"]),
        ('l', None, &["iang", "uang"]),
    ],
    &[
        ('z', None, &["ou"]),
        ('x', None, &["ia", "ua"]),
        ('c', None, &["ao"]),
        ('v', Some("zh"), &["ui", "ü"]),
        ('b', None, &["in"]),
        ('n', None, &["iao"]),
        ('m', None, &["ian"]),
    ],
];

/// 小鹤敲了 `v`（zh）之后各键拼成的音节；不在表里的键拼不出。
const XIAOHE_AFTER_V: [(char, &str); 20] = [
    ('w', "zhei"),
    ('e', "zhe"),
    ('r', "zhuan"),
    ('y', "zhun"),
    ('u', "zhu"),
    ('i', "zhi"),
    ('o', "zhuo"),
    ('a', "zha"),
    ('s', "zhong"),
    ('d', "zhai"),
    ('f', "zhen"),
    ('g', "zheng"),
    ('h', "zhang"),
    ('j', "zhan"),
    ('k', "zhuai"),
    ('l', "zhuang"),
    ('z', "zhou"),
    ('x', "zhua"),
    ('c', "zhao"),
    ('v', "zhui"),
];

/// 键位提示样例：空闲的整张表，与敲了 zh 之后。
pub fn keyboard_scenes() -> Vec<(&'static str, Keyboard)> {
    vec![
        ("keyboard-idle", xiaohe(None)),
        ("keyboard-zh", xiaohe(Some(&XIAOHE_AFTER_V))),
        ("keyboard-wubi", wubi()),
    ]
}

/// 86 版五笔字根图，读随包的字形表（`assets/schemes/wubi86/roots86-glyphs.tsv`）。
fn wubi() -> Keyboard {
    const TABLE: &str = include_str!("../../../../assets/schemes/wubi86/roots86-glyphs.tsv");
    let mut keys: Vec<(char, Keycap)> = Vec::new();
    for line in TABLE.lines().filter(|line| !line.starts_with('#')) {
        let fields: Vec<&str> = line.split('\t').collect();
        let key = fields[0].chars().next().unwrap();
        let frame: Vec<f32> = fields[5]
            .split(' ')
            .filter_map(|v| v.parse().ok())
            .collect();
        let frame = match frame[..] {
            [left, top, right, bottom] => Some([left, top, right, bottom]),
            _ => None,
        };
        let root = Root::from_svg(fields[6], frame).unwrap();
        match keys.iter_mut().find(|(k, _)| *k == key) {
            Some((_, cap)) => cap.roots.push(root),
            None => keys.push((
                key,
                Keycap {
                    label: key.to_string(),
                    corner: Some(fields[1].to_owned()),
                    roots: vec![root],
                    ..Keycap::default()
                },
            )),
        }
    }
    let rows = ["QWERTYUIOP", "ASDFGHJKL", "ZXCVBNM"]
        .iter()
        .map(|row| {
            row.chars()
                .map(|key| match keys.iter().find(|(k, _)| *k == key) {
                    Some((_, cap)) => cap.clone(),
                    None => Keycap {
                        label: key.to_string(),
                        lines: vec!["万能键".to_owned()],
                        ..Keycap::default()
                    },
                })
                .collect()
        })
        .collect();
    Keyboard { rows }
}

fn xiaohe(next: Option<&[(char, &str)]>) -> Keyboard {
    let rows = XIAOHE
        .iter()
        .map(|row| {
            row.iter()
                .map(|(key, corner, finals)| {
                    let state = match next {
                        None => KeycapState::Normal,
                        Some(next) => next
                            .iter()
                            .find(|(k, _)| k == key)
                            .map_or(KeycapState::Dimmed, |(_, syllable)| {
                                KeycapState::Next((*syllable).to_owned())
                            }),
                    };
                    Keycap {
                        label: key.to_ascii_uppercase().to_string(),
                        corner: corner.map(str::to_owned),
                        lines: finals.iter().map(|f| (*f).to_owned()).collect(),
                        state,
                        roots: Vec::new(),
                    }
                })
                .collect()
        })
        .collect();
    Keyboard { rows }
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

/// 横排展开成矩阵：三行五格，序号只标在高亮那一行，末行补空位；高亮的候选在下面出译文。
pub fn matrix() -> Frame {
    let words = [
        "是", "时", "事", "使", "式", "十", "实", "市", "石", "世", "视", "试", "示",
    ];
    let mut rows: Vec<Row> = words
        .iter()
        .enumerate()
        .map(|(i, word)| {
            let mut row = if *word == "实" {
                annotated(
                    i % 5,
                    word,
                    &[("adj. ", Tone::Pos), ("real", Tone::Gloss)],
                    false,
                )
            } else {
                Row::plain(i % 5, *word)
            };
            if i / 5 != 1 {
                row.index = String::new();
            }
            row
        })
        .collect();
    rows.resize(15, Row::plain(0, ""));
    for row in &mut rows[words.len()..] {
        row.index = String::new();
    }
    Frame {
        preedit: Some(Preedit::plain("shi", 3)),
        rows,
        highlighted: Some(6),
        footer: Some("2/9".to_owned()),
        sentence: None,
        status: None,
        mode: Mode::default(),
        columns: 5,
        column_ems: vec![1.0; 5],
    }
}
