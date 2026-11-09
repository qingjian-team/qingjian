//! 候选会话的分页、高亮与矩阵导航测试。

use super::*;
use qingjian_core::CandidateKind;

fn candidates(count: usize) -> Vec<Candidate> {
    (0..count)
        .map(|i| Candidate {
            text: format!("本{i}"),
            kind: CandidateKind::Chinese,
            syllables: vec!["a".into()],
            reading: None,
            translation: None,
            aux_code: None,
        })
        .collect()
}

#[test]
fn can_turn_page_tells_whether_the_page_would_move() {
    let mut session = Session::default();
    // 10 条、每页 5：首页往下翻得动、往上翻不动（翻页键「翻不动就落字」靠它）
    session.reset(None, candidates(10), 5, 0);
    assert_eq!(session.page, 0);
    assert!(session.can_turn_page(1));
    assert!(!session.can_turn_page(-1));
    assert!(session.turn_page(1));
    assert_eq!(session.page, 1);
    // 末页反过来
    assert!(session.can_turn_page(-1));
    assert!(!session.can_turn_page(1));

    // 只有一页、以及没有候选：两个方向都翻不动
    for count in [3, 0] {
        session.reset(None, candidates(count), 5, 0);
        assert!(!session.can_turn_page(1));
        assert!(!session.can_turn_page(-1));
    }
}

#[test]
fn vertical_keys_expand_the_row_into_a_scrolling_grid() {
    let mut session = Session::default();
    session.reset(None, candidates(100), 5, 0);
    assert!(session.grid_cells().is_none());
    // 左右键在单行时不归候选管
    assert!(!session.move_cells(1));
    // 第一下往下：展开并换到第二行同一列，数字键跟着选第二行
    assert!(session.move_rows(1));
    assert_eq!((session.highlighted, session.page), (5, 1));
    assert_eq!(session.index_on_page(2), Some(7));
    let (cells, columns) = session.grid_cells().unwrap();
    assert_eq!((cells.len(), columns), (30, 5));
    // 展开后左右键按阅读顺序移动，越过行尾到下一行
    assert!(session.move_cells(-1));
    assert_eq!((session.highlighted, session.page), (4, 0));
    // 一直往下：视口按行滚动
    for _ in 0..7 {
        session.move_rows(1);
    }
    assert_eq!(session.page, 7);
    assert_eq!(session.grid.unwrap().top(), 2);
    // 翻页键一次一屏；Esc 收回单行，高亮不动
    assert!(session.move_screens(1));
    assert_eq!(session.page, 13);
    assert!(session.collapse());
    assert!(!session.collapse());
    assert_eq!(session.page, 13);
    // 新一轮查询回到单行
    session.move_rows(1);
    session.reset(None, candidates(3), 5, 0);
    assert!(session.grid.is_none());
    // 只有一行时往上：展开但不动，仍要重画
    assert!(session.move_rows(-1));
    let (cells, _) = session.grid_cells().unwrap();
    assert_eq!(cells.len(), 5);
}

/// ← 顶在第一个候选上移不动（归拼音光标，由控制器分派）；→ 顶在最后一个候选上不动，矩阵保持展开。
#[test]
fn arrows_at_the_grid_edges_do_not_move() {
    let mut session = Session::default();
    session.reset(None, candidates(20), 5, 0);
    session.move_rows(1);
    // ← 一路走回第一个候选，全程还是矩阵里移动
    for _ in 0..5 {
        assert!(session.move_cells(-1));
        assert!(session.grid.is_some());
    }
    assert_eq!(session.highlighted, 0);
    // 顶在第一个候选上再按 ←：移不动，矩阵保持展开（这一下由控制器交还拼音光标）
    assert!(!session.move_cells(-1));
    assert!(session.grid.is_some());
    assert_eq!(session.highlighted, 0);
    // → 顶在最后一个候选上移不动：矩阵保持展开
    session.move_rows(1);
    while session.move_cells(1) {}
    assert_eq!(session.highlighted, 19);
    assert!(session.grid.is_some());
}

/// 矩阵里顶在第一排再按 ↑：高亮回到第一个候选、视口滚回顶部；已经在第一个候选就不动。
#[test]
fn up_at_the_first_row_jumps_back_to_the_first_candidate() {
    let mut session = Session::default();
    session.reset(None, candidates(40), 5, 0);
    session.move_rows(1);
    session.move_cells(1);
    assert_eq!(session.highlighted, 6);
    // 回第一排第二格
    assert!(session.move_rows(-1));
    assert_eq!(session.highlighted, 1);
    // 第一排再往上：回到第一个候选
    assert!(session.jump_to_first());
    assert_eq!((session.highlighted, session.page), (0, 0));
    assert_eq!(session.grid.unwrap().top(), 0);
    // 已在第一个候选：不动
    assert!(!session.jump_to_first());
}

/// 给方向键规则的矩阵状态：没开是 Off、单行是 Closed，展开后看高亮在不在第一行；整段输入结束焦点回到候选。
#[test]
fn matrix_state_and_focus_follow_the_session() {
    let mut session = Session::default();
    session.reset(None, candidates(40), 5, 0);
    assert_eq!(session.matrix(false), Matrix::Off);
    assert_eq!(session.matrix(true), Matrix::Closed);
    session.move_rows(1);
    assert_eq!(session.matrix(true), Matrix::Open { first_row: false });
    session.move_rows(-1);
    assert_eq!(session.matrix(true), Matrix::Open { first_row: true });
    session.focus = Focus::Pinyin;
    session.reset(None, candidates(40), 5, 0);
    assert_eq!(session.focus, Focus::Candidates);
}

#[test]
fn clicks_in_an_expanded_grid_map_to_viewport_cells() {
    let mut session = Session::default();
    session.reset(None, candidates(100), 5, 0);
    assert_eq!(session.index_in_view(3), Some(3));
    for _ in 0..7 {
        session.move_rows(1);
    }
    let top = session.grid.unwrap().top();
    assert!(top > 0);
    // 视口第二行第三格：不是当前页的第 8 格
    assert_eq!(session.index_in_view(7), Some(top * 5 + 7));
    assert_ne!(session.index_in_view(7), session.index_on_page(7));
}

#[test]
fn digits_map_to_cells_on_the_current_page() {
    let mut session = Session::default();
    session.reset(None, candidates(3), 9, 2);
    assert_eq!(session.index_on_page(2), Some(2));
    assert_eq!(session.index_on_page(3), None);
    assert!(session.move_highlight(1));
    assert!(session.move_highlight(5));
    assert_eq!(session.highlighted, 2);
    assert!(!session.move_highlight(1));
}

#[test]
fn page_edges_stay_on_current_page_and_skip_empty_cells() {
    let mut session = Session::default();
    session.reset(None, candidates(12), 5, 0);
    assert!(session.move_to_page_edge(true));
    assert_eq!((session.page, session.highlighted), (0, 4));
    assert!(!session.move_to_page_edge(true));
    assert!(session.turn_page(1));
    assert!(session.move_to_page_edge(true));
    assert_eq!((session.page, session.highlighted), (1, 9));
    assert!(session.move_to_page_edge(false));
    assert_eq!((session.page, session.highlighted), (1, 5));
    assert!(session.turn_page(1));
    assert!(session.move_to_page_edge(true));
    assert_eq!((session.page, session.highlighted), (2, 11));
    assert!(session.navigated);

    session.reset(None, candidates(20), 5, 0);
    assert!(session.move_rows(1));
    assert!(session.move_to_page_edge(true));
    assert_eq!((session.page, session.highlighted), (1, 9));
    assert!(session.grid.is_some());
    assert!(session.move_to_page_edge(false));
    assert_eq!((session.page, session.highlighted), (1, 5));

    let mut words = candidates(1);
    words[0].kind = CandidateKind::Custom(9);
    session.reset(None, words, 5, 2);
    assert_eq!((session.page, session.highlighted), (1, 8));
    assert!(!session.move_to_page_edge(false));
    assert!(!session.move_to_page_edge(true));
    let mut sparse = candidates(2);
    sparse[0].kind = CandidateKind::Custom(7);
    sparse[1].kind = CandidateKind::Custom(9);
    session.reset(None, sparse, 5, 2);
    assert_eq!((session.page, session.highlighted), (1, 6));
    assert!(session.move_to_page_edge(true));
    assert_eq!(session.highlighted, 8);
    assert!(session.move_to_page_edge(false));
    assert_eq!(session.highlighted, 6);
    session.reset(None, Vec::new(), 5, 0);
    assert!(!session.move_to_page_edge(false));
    assert!(!session.move_to_page_edge(true));
}

#[test]
fn paging_follows_the_layout() {
    let mut session = Session::default();
    session.reset(None, candidates(12), 9, 2);
    assert_eq!(session.pages(), 2);
    assert!(session.turn_page(1));
    assert_eq!(session.highlighted, 9);
    assert_eq!(session.index_on_page(0), Some(9));
    assert!(!session.turn_page(1));
}

#[test]
fn navigation_is_remembered_until_the_next_query() {
    let mut session = Session::default();
    session.reset(None, candidates(12), 9, 2);
    assert!(!session.navigated);
    // 顶到边界没动算没导航
    assert!(!session.move_highlight(-1));
    assert!(!session.navigated);
    assert!(session.move_highlight(1));
    assert!(session.navigated);
    session.reset(None, candidates(3), 9, 2);
    assert!(!session.navigated);
    // 只有一页时翻页没动，也不算导航
    assert!(!session.turn_page(1));
    assert!(!session.navigated);
    session.reset(None, candidates(12), 9, 2);
    assert!(session.turn_page(1));
    assert!(session.navigated);
}

#[test]
fn sparse_custom_positions_keep_highlight_on_real_candidates() {
    let mut words = candidates(1);
    words[0].kind = CandidateKind::Custom(9);
    let mut session = Session::default();
    session.reset(None, words.clone(), 5, 2);
    assert_eq!((session.page, session.highlighted), (1, 8));
    assert!(!session.turn_page(-1));
    words.extend(candidates(1));
    session.reset(None, words, 5, 2);
    assert_eq!((session.page, session.highlighted), (0, 0));
    assert!(session.turn_page(1));
    assert_eq!((session.page, session.highlighted), (1, 8));
    assert!(session.move_highlight(-1));
    assert_eq!((session.page, session.highlighted), (0, 0));
}
