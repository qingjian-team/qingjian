//! 全拼、双拼、直输与编辑状态不应借用候选显示串。
use super::assert_raw;
use crate::dictionary::Dictionary;
use crate::engine::tests::engine;
use crate::{CandidateKind, ShuangpinScheme};

#[test]
fn raw_preedit_preserves_typed_text_without_automatic_separators() {
    for input in ["", "nihao", "xi'an", "no-Way", "gpt-6", "v1+2"] {
        let mut engine = engine();
        engine.set_input(input);
        let _ = engine.query();
        assert_raw(&mut engine, input, input.len());
    }
    let mut engine = engine();
    engine.set_shift_letter_compose(true);
    engine.set_input("NiHao");
    assert_eq!(engine.composition().text(), "nihao");
    assert_raw(&mut engine, "NiHao", 5);
}

#[test]
fn raw_preedit_keeps_shuangpin_keys_and_english_case() {
    let mut engine = engine();
    engine.set_shuangpin(Some(ShuangpinScheme::Xiaohe));
    engine.set_input("nihc");
    assert_eq!(engine.query().unwrap().marked_text(), "ni'hao");
    assert_raw(&mut engine, "nihc", 4);
    engine.set_english_mode(true);
    engine.set_input("OpenAI");
    assert_raw(&mut engine, "OpenAI", 6);
}

#[test]
fn raw_preedit_keeps_suffix_after_editing_and_excludes_auxiliary_codes() {
    let mut engine = engine();
    engine.set_input("nihao");
    for _ in 0..3 {
        engine.move_cursor_left();
    }
    engine.push('x');
    engine.delete_forward();
    assert_raw(&mut engine, "nixao", 3);
    engine.set_input("kaifa");
    engine.enter_aux();
    assert!(engine.push_aux_code('k'));
    assert_raw(&mut engine, "kaifa", 5);
}

#[test]
fn raw_preedit_only_contains_the_uncommitted_remainder() {
    let mut engine = engine();
    engine.set_input("kaifazhe");
    let candidate = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == "开发" && c.kind == CandidateKind::Chinese)
        .unwrap();
    assert_eq!(engine.commit(&candidate), "开发");
    assert_raw(&mut engine, "zhe", 3);
}

#[test]
fn raw_preedit_cursor_covers_home_middle_and_end() {
    for position in 0..=5 {
        let mut engine = engine();
        engine.set_input("nihao");
        engine.move_cursor_home();
        for _ in 0..position {
            engine.move_cursor_right();
        }
        assert_raw(&mut engine, "nihao", position);
    }
}

#[test]
fn raw_preedit_does_not_accept_or_record_a_spelling_correction() {
    let dictionary = Dictionary::parse("你好吗\tni hao ma\t50000\n你好\tni hao\t10000\n").unwrap();
    let mut engine = crate::Engine::new(dictionary);
    engine.set_input("nihooma");
    assert!(engine.query().unwrap().correction.is_some());
    assert_raw(&mut engine, "nihooma", 7);
}
