//! 混输的满码唯一上屏与形码优先续输，保持旧配置的拼音行为。

use qingjian_dictionary::CodeTable;

use crate::{CandidateKind, CodeScheme, CustomPhrase, Engine};

use super::{CODES, mixed};

fn configured() -> Engine {
    let mut engine = mixed();
    let source = format!("{CODES}甲\taaaa\t10\n乙\taaaa\t5\n");
    let mut scheme = CodeScheme::new("wubi", CodeTable::parse(&source).unwrap());
    scheme.code_length = 4;
    scheme.mixed_auto_commit = true;
    scheme.mixed_top = true;
    engine.set_code_scheme(Some(scheme));
    engine
}

#[test]
fn unique_short_codes_never_commit_automatically() {
    let mut engine = configured();
    for keys in ["v", "gc", "kai", "ggl", "aaaa", "ggllg"] {
        engine.set_input(keys);
        assert_eq!(engine.commit_unique_code(), None, "{keys}");
        assert_eq!(engine.composition().text(), keys);
    }
    engine.set_input("ggll");
    assert_eq!(engine.commit_unique_code().as_deref(), Some("一"));
    assert!(engine.composition().is_empty());
}

#[test]
fn topping_needs_a_full_code_with_a_match_and_a_letter() {
    let mut engine = configured();
    for keys in ["v", "gc", "kai", "zzzz"] {
        engine.set_input(keys);
        assert!(!engine.code_tops('g'), "{keys}");
    }
    engine.set_input("aaaa");
    assert!(engine.code_tops('g'));
    for character in ['1', ',', 'G'] {
        assert!(!engine.code_tops(character));
    }
    assert!(!engine.composing_code());
}

#[test]
fn mixed_top_prioritizes_code_prefixes_without_changing_the_default() {
    let mut engine = configured();
    engine.set_input("ka");
    assert_eq!(
        engine.query().unwrap().candidates.items[0].kind,
        CandidateKind::Code
    );
    engine.set_mixed_code_rules(true, false);
    assert_ne!(
        engine.query().unwrap().candidates.items[0].kind,
        CandidateKind::Code
    );
}

#[test]
fn mixed_switches_do_not_control_pure_code_rules() {
    let mut engine = configured();
    engine.set_phonetic(false);
    engine.set_input("ggll");
    assert_eq!(engine.commit_unique_code(), None);
    assert!(!engine.code_tops('g'));
    engine.set_code_rules(true, true);
    assert_eq!(engine.commit_unique_code().as_deref(), Some("一"));
}

#[test]
fn non_code_states_do_not_commit_or_top() {
    let mut engine = configured();
    engine.set_input("ggll");
    engine.move_cursor_left();
    assert_eq!(engine.commit_unique_code(), None);
    assert!(!engine.code_tops('g'));
    engine.move_cursor_end();
    engine.set_english_mode(true);
    assert_eq!(engine.commit_unique_code(), None);
    assert!(!engine.code_tops('g'));
    engine.set_english_mode(false);
    for keys in ["ggLL", "gg-ll", "ggll?"] {
        engine.set_input(keys);
        assert_eq!(engine.commit_unique_code(), None);
        assert!(!engine.code_tops('g'));
    }
    engine.set_input("kaifa");
    engine.enter_aux();
    assert_eq!(engine.commit_unique_code(), None);
    assert!(!engine.code_tops('g'));
}

#[test]
fn a_custom_phrase_before_the_code_requires_explicit_selection() {
    let mut engine = configured();
    engine
        .set_custom_phrases(vec![CustomPhrase::new("ggll", "自定义")])
        .unwrap();
    engine.set_input("ggll");
    assert_eq!(engine.commit_unique_code(), None);
}

#[test]
fn learned_code_words_also_count_against_uniqueness() {
    use crate::{Candidate, Learner};

    struct CodeWords(CodeTable);

    impl Learner for CodeWords {
        fn record(&mut self, _candidate: &Candidate) {}

        fn weight(&self, _text: &str) -> u32 {
            0
        }

        fn user_code_words(&self, scheme: &str) -> Option<&CodeTable> {
            (scheme == "wubi").then_some(&self.0)
        }
    }

    let mut engine = configured().with_learner(Box::new(CodeWords(
        CodeTable::parse("新词\tggll\t1\n用户词\txyzz\t1\n").unwrap(),
    )));
    engine.set_input("ggll");
    assert_eq!(engine.commit_unique_code(), None);
    assert_eq!(engine.composition().text(), "ggll");
    engine.set_input("xyzz");
    assert_eq!(engine.commit_unique_code().as_deref(), Some("用户词"));
}
