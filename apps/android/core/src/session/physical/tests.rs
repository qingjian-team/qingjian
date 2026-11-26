//! 物理键盘会话回归：输入、导航、页码、编辑与透传边界。

use crate::frame::{Frame, FrameCandidate};
use crate::session::{PhysicalPage, Session};
use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use std::path::PathBuf;

fn session() -> Session {
    let dict =
        Dictionary::parse("你好\tni hao\t1000\n你\tni\t900\n好\thao\t800\n拟好\tni hao\t700\n")
            .unwrap();
    Session::new(
        Engine::new(dict),
        PathBuf::new(),
        PathBuf::new(),
        PathBuf::new(),
    )
}

fn press(
    session: &mut Session,
    action: i32,
    character: Option<char>,
    repeat: bool,
) -> Option<Frame> {
    let frame = session.physical_key(action, character, repeat)?;
    session.note_physical(&frame);
    Some(frame)
}

fn type_text(session: &mut Session, text: &str) -> Frame {
    let mut frame = None;
    for character in text.chars() {
        frame = press(session, 0, Some(character), false);
    }
    frame.unwrap()
}

#[test]
fn space_selects_and_enter_commits_raw() {
    let mut s = session();
    assert!(
        type_text(&mut s, "nihao")
            .candidates
            .iter()
            .any(|c| c.text == "你好")
    );
    assert_eq!(press(&mut s, 1, None, false).unwrap().commit, "你好");
    type_text(&mut s, "nihao");
    assert_eq!(press(&mut s, 2, None, false).unwrap().commit, "nihao");
}

#[test]
fn editing_keys_pass_through_outside_composition() {
    let mut s = session();
    for action in 1..=13 {
        assert!(
            press(&mut s, action, None, false).is_none(),
            "action {action}"
        );
    }
    assert!(press(&mut s, 0, Some('3'), false).is_none());
    assert!(press(&mut s, 0, Some('A'), false).is_none());
}

#[test]
fn held_backspace_stops_after_composition_but_fresh_press_passes() {
    let mut s = session();
    type_text(&mut s, "n");
    assert!(press(&mut s, 3, None, false).unwrap().preedit.is_empty());
    assert!(press(&mut s, 3, None, true).is_some());
    assert!(press(&mut s, 3, None, false).is_none());
    assert!(press(&mut s, 3, None, true).is_none());
}

#[test]
fn arrows_edit_engine_caret_and_escape_clears() {
    let mut s = session();
    type_text(&mut s, "nihao");
    let moved = press(&mut s, 6, None, false).unwrap();
    assert!(moved.preedit_cursor < moved.preedit.chars().count());
    let cleared = press(&mut s, 5, None, false).unwrap();
    assert!(cleared.preedit.is_empty() && cleared.commit.is_empty());
}

#[test]
fn expression_digits_are_input_before_candidate_selection() {
    let mut s = session();
    type_text(&mut s, "v1+2");
    assert_eq!(press(&mut s, 2, None, false).unwrap().commit, "v1+2");
}

#[test]
fn page_numbers_and_highlight_match_the_current_page() {
    let mut s = session();
    type_text(&mut s, "ni");
    let mut frame = s.frame(String::new());
    frame.candidates = (0..12)
        .map(|i| FrameCandidate::plain(&format!("词{i}")))
        .collect();
    let mut page = PhysicalPage::default();
    page.show(&frame);
    assert_eq!(page.index(0), Some(0));
    page.turn(1);
    assert_eq!(page.index(0), Some(5));
    assert!(page.previous() && page.next());
    page.step(1);
    assert_eq!(page.selected(), 6);
    page.turn(1);
    assert_eq!(page.index(1), Some(11));
    assert_eq!(page.index(2), None);
    assert!(!page.next());
    page.show(&frame);
    assert_eq!(page.selected(), 10);
    frame.preedit.push('a');
    page.show(&frame);
    assert_eq!(page.selected(), 0);
}

#[test]
fn no_candidate_space_falls_back_to_raw_and_navigation_preserves_metadata() {
    let mut s = session();
    type_text(&mut s, "zzzzzz");
    s.physical.items.clear();
    assert_eq!(press(&mut s, 1, None, false).unwrap().commit, "zzzzzz");
    let mut frame = s.frame(String::new());
    frame.expression = true;
    frame.single_chars = true;
    frame.pinyin_choices = vec!["ni".to_owned()];
    s.physical.show(&frame);
    let shown = s.physical.frame();
    assert!(shown.expression && shown.single_chars);
    assert_eq!(shown.pinyin_choices, ["ni"]);
}

#[test]
fn touch_completions_do_not_leak_into_physical_input() {
    let mut s = session();
    let frame = s.complete("name@");
    assert!(!frame.candidates.is_empty());
    s.note_physical(&frame);
    assert!(press(&mut s, 11, None, false).is_none());
    assert!(press(&mut s, 0, Some('1'), false).is_none());
    assert!(s.complete("name@").candidates.is_empty());
}

#[test]
fn numeric_selection_uses_visible_slot_and_shift_mode_letter_keeps_core_semantics() {
    let mut s = session();
    let frame = type_text(&mut s, "nihao");
    let expected = frame.candidates[0].text.clone();
    assert_eq!(press(&mut s, 0, Some('1'), false).unwrap().commit, expected);
    s.set_shuangpin("xiaohe", false);
    assert!(
        press(&mut s, 0, Some('V'), false)
            .unwrap()
            .preedit
            .starts_with('V')
    );
}

#[test]
fn zhuyin_digits_select_after_navigation_and_caps_pass_through_in_chinese() {
    let mut s = session();
    s.set_zhuyin(true);
    type_text(&mut s, "su3");
    let frame = press(&mut s, 13, None, false).unwrap();
    let expected = frame.candidates[s.physical.selected()].text.clone();
    assert_eq!(
        press(&mut s, 0, Some('1'), false).unwrap().commit,
        frame.candidates[0].text
    );
    assert!(!expected.is_empty());
    s.set_zhuyin(false);
    type_text(&mut s, "ni");
    assert!(press(&mut s, 0, Some('A'), false).is_none());
}
