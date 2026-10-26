//! 帧里的候选换成候选条要画的东西：译文只留第一条、长译文截断、emoji 与云端标记。

use super::*;
use crate::frame::FrameSense;

/// 一个字符算 1 点宽，量得准、好算。
fn chars(text: &str, _size: f32) -> f32 {
    text.chars().count() as f32
}

fn candidate(text: &str, senses: &[(Option<&'static str>, &str, bool)]) -> FrameCandidate {
    FrameCandidate {
        text: text.to_owned(),
        senses: senses
            .iter()
            .map(|&(pos, text, fresh)| FrameSense {
                pos,
                text: text.to_owned(),
                reading: None,
                fresh,
            })
            .collect(),
        forgettable: true,
        cloud: false,
    }
}

#[test]
fn only_the_first_sense_goes_under_the_word() {
    let rows = rows(
        &[
            candidate(
                "你好",
                &[(Some("int."), "hello", false), (None, "hi", false)],
            ),
            candidate("你好像", &[(Some("phr."), "you seem", true)]),
            candidate("你哈", &[]),
        ],
        1.0,
        chars,
    );
    assert_eq!(rows[0].index, "");
    assert_eq!(
        rows[0].annotation,
        vec![
            ("int. ".to_owned(), Tone::Pos),
            ("hello".to_owned(), Tone::Gloss)
        ]
    );
    assert_eq!(rows[1].annotation[1], ("you seem".to_owned(), Tone::Fresh));
    assert!(rows[2].annotation.is_empty());
}

#[test]
fn long_glosses_are_cut_with_an_ellipsis() {
    assert_eq!(fit("hello", 5.0, |t| chars(t, 0.0)), "hello");
    assert_eq!(fit("hello world", 5.0, |t| chars(t, 0.0)), "hell…");
    assert_eq!(fit("hello", 0.0, |t| chars(t, 0.0)), "…");
}

#[test]
fn emoji_candidates_are_flagged() {
    assert!(has_emoji("👋"));
    assert!(has_emoji("❤️"));
    assert!(has_emoji("🇨🇳"));
    assert!(has_emoji("⌛"));
    assert!(has_emoji("⌚"));
    assert!(has_emoji("⏰"));
    assert!(!has_emoji("你好"));
    assert!(!has_emoji("hello"));
}

#[test]
fn cloud_candidates_carry_the_cloud_mark() {
    let mut cloud = candidate("空间改造方案", &[]);
    cloud.cloud = true;
    let rows = rows(&[candidate("空间", &[]), cloud], 1.0, chars);
    assert!(!rows[0].cloud);
    assert!(rows[1].cloud);
}
