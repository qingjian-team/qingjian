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

#[test]
fn lattice_keeps_complete_boundaries_and_prunes_mid_word_initials() {
    // 78746878326 (ru shou qu dao)：存在完整路径时，中间的 7 不拆成未选定单码简拼
    let (dag, tail) = lattice("78746878326");
    assert!(tail.is_empty());
    assert!(dag.has_complete_path());
    assert!(dag.has_multi_syllable_path());
    assert!(
        dag.edges_at(2)
            .iter()
            .all(|edge| edge.position.complete || edge.position.pinned || edge.next == dag.end()),
        "完整路径下中间位置不应保留未选定单码简拼：{:?}",
        dag.edges_at(2)
    );
    // 保留全部合法完整切法边界（584826 同时含 58+4826 与 584+826）
    let (dag, _) = lattice("584826");
    let first_lens: Vec<usize> = dag.edges_at(0).iter().map(|e| e.position.len).collect();
    assert!(
        first_lens.contains(&2) && first_lens.contains(&3),
        "{first_lens:?}"
    );
}

#[test]
fn lattice_keeps_abbreviations_when_no_complete_path_exists() {
    // 57 (j/k/l + p/q/r/s) 无法全由完整音节覆盖：保留全部单码简拼边
    let (dag, tail) = lattice("57");
    assert!(tail.is_empty());
    assert!(!dag.has_complete_path());
    assert_eq!(dag.edges_at(0).len(), 1);
    assert_eq!(dag.edges_at(0)[0].next, 1);
    assert!(!dag.edges_at(0)[0].position.complete);
    let best = dag.best_shape().unwrap();
    assert_eq!(best.positions.len(), 2);
}

#[test]
fn lattice_skips_separators_and_handles_unreadable_tail() {
    let (dag, tail) = lattice("64'");
    assert!(tail.is_empty());
    assert_eq!(dag.end(), 3);
    assert!(!dag.edges_at(0)[0].position.prefix);

    let (dag, tail) = lattice("6410");
    assert_eq!(dag.end(), 2);
    assert_eq!(tail, "10");

    let (dag, tail) = lattice("ni'426");
    assert!(tail.is_empty());
    assert_eq!(dag.edges_at(0)[0].next, 3);
    assert!(dag.edges_at(0)[0].position.pinned);
}
