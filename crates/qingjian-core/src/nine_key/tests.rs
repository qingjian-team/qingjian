//! 数字串切成形状：完整音节、末尾前缀、选定字母、简拼与切不动的尾巴。

use super::*;

fn alternatives(shape: &Shape) -> Vec<Vec<&str>> {
    shape
        .positions
        .iter()
        .map(|p| p.alternatives.iter().map(String::as_str).collect())
        .collect()
}

#[test]
fn digits_split_into_syllable_positions() {
    let (shapes, tail) = segment("64426");
    assert!(tail.is_empty());
    let best = alternatives(&shapes[0]);
    assert_eq!(best.len(), 2);
    assert!(best[0].contains(&"ni") && best[0].contains(&"mi"));
    assert!(best[1].contains(&"hao"));
}

#[test]
fn last_position_reads_as_prefix() {
    let (shapes, _) = segment("942");
    let last = shapes[0].positions.last().unwrap();
    assert!(last.prefix);
    assert!(last.alternatives.iter().any(|a| a == "xia"));
}

#[test]
fn pinned_letters_leave_one_alternative() {
    let (shapes, tail) = segment("ni'426");
    assert!(tail.is_empty());
    assert_eq!(alternatives(&shapes[0])[0], vec!["ni"]);
    assert_eq!(shapes[0].positions[1].start, 3);
}

#[test]
fn initials_read_as_abbreviation() {
    // 5 7：j/k/l + p/q/r/s，没有哪个音节或前缀的码是 57
    let (shapes, tail) = segment("57");
    assert!(tail.is_empty());
    assert_eq!(shapes[0].positions.len(), 2);
    assert!(!shapes[0].positions[0].complete);
}

#[test]
fn trailing_separator_is_skipped() {
    let (shapes, tail) = segment("64'");
    assert_eq!(shapes[0].end(), 2);
    assert!(tail.is_empty());
}

#[test]
fn unreadable_tail_is_returned() {
    let (shapes, tail) = segment("6410");
    assert_eq!(shapes[0].end(), 2);
    assert_eq!(tail, "10");
}
