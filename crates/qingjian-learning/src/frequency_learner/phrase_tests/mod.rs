//! 形码用户词的隔离、保存和 Core 上屏联调。

mod boundaries;
mod storage;

use super::FrequencyLearner;
use qingjian_core::{CodeScheme, Engine};
use qingjian_dictionary::{CodeTable, Dictionary};

fn engine() -> Engine {
    let table = CodeTable::parse(
        "甲\ta\t100\n甲\tabcd\t10\n乙\tefgh\t10\n丙\tijkl\t10\n丁\tmnop\t10\n戊\tqrst\t10\n",
    )
    .unwrap();
    let mut scheme = CodeScheme::new("wubi", table);
    scheme.wubi_phrases = true;
    let mut engine = Engine::new(Dictionary::parse("开\tkai\t1\n").unwrap())
        .with_learner(Box::<FrequencyLearner>::default());
    engine.set_code_scheme(Some(scheme));
    engine.set_phonetic(false);
    engine
}

fn commit(engine: &mut Engine, code: &str, text: &str) {
    engine.set_input(code);
    let candidate = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|candidate| candidate.text == text)
        .unwrap();
    assert_eq!(engine.commit(&candidate), text);
}

fn has_word(engine: &mut Engine, code: &str, text: &str) -> bool {
    engine.set_input(code);
    engine
        .query()
        .unwrap()
        .candidates
        .items
        .iter()
        .any(|candidate| candidate.text == text)
}

#[test]
fn wubi_learns_on_first_continuous_input_and_uses_full_codes() {
    let mut engine = engine();
    commit(&mut engine, "a", "甲");
    commit(&mut engine, "efgh", "乙");
    assert!(has_word(&mut engine, "abef", "甲乙"));
    engine.set_auto_phrase(false, 4);
    assert!(has_word(&mut engine, "abef", "甲乙"));
}

#[test]
fn wubi_respects_switch_privacy_learning_and_punctuation() {
    for boundary in ["disabled", "private", "learning", "punctuation", "focus"] {
        let mut engine = engine();
        if boundary == "disabled" {
            engine.set_auto_phrase(false, 4);
        }
        if boundary == "private" {
            engine.set_private(true);
        }
        if boundary == "learning" {
            engine.set_learning(false);
        }
        commit(&mut engine, "a", "甲");
        if boundary == "punctuation" {
            engine.punctuate('，');
        }
        if boundary == "focus" {
            engine.break_chain();
        }
        commit(&mut engine, "efgh", "乙");
        assert!(!has_word(&mut engine, "abef", "甲乙"), "{boundary}");
    }
}

#[test]
fn wubi_max_length_is_characters_and_rolling_window_is_bounded() {
    let mut engine = engine();
    engine.set_auto_phrase(true, 10);
    for (code, text) in [
        ("a", "甲"),
        ("efgh", "乙"),
        ("ijkl", "丙"),
        ("mnop", "丁"),
        ("qrst", "戊"),
        ("a", "甲"),
        ("efgh", "乙"),
        ("ijkl", "丙"),
        ("mnop", "丁"),
        ("qrst", "戊"),
    ] {
        commit(&mut engine, code, text);
    }
    assert!(has_word(&mut engine, "aeiq", "甲乙丙丁戊甲乙丙丁戊"));
    engine.set_input("");
    commit(&mut engine, "a", "甲");
    assert!(!has_word(&mut engine, "aeia", "甲乙丙丁戊甲乙丙丁戊甲"));

    let mut engine = self::engine();
    engine.set_auto_phrase(true, 2);
    commit(&mut engine, "a", "甲");
    commit(&mut engine, "efgh", "乙");
    commit(&mut engine, "ijkl", "丙");
    assert!(!has_word(&mut engine, "aeij", "甲乙丙"));
    assert!(has_word(&mut engine, "efij", "乙丙"));
}

#[test]
fn wubi_mixed_input_and_scheme_changes_break_phrase_source() {
    let mut engine = engine();
    engine.set_phonetic(true);
    commit(&mut engine, "a", "甲");
    commit(&mut engine, "kai", "开");
    commit(&mut engine, "efgh", "乙");
    assert!(!has_word(&mut engine, "abef", "甲乙"));
    assert!(!has_word(&mut engine, "aeg", "甲开乙"));

    engine.break_chain();
    commit(&mut engine, "a", "甲");
    let scheme = engine.code_scheme().unwrap().clone();
    engine.set_code_scheme(Some(scheme));
    commit(&mut engine, "efgh", "乙");
    assert!(!has_word(&mut engine, "abef", "甲乙"));
}

#[test]
fn wubi_edit_and_privacy_exit_do_not_reuse_earlier_input() {
    for boundary in ["backspace", "edit", "private-exit", "learning-exit"] {
        let mut engine = engine();
        commit(&mut engine, "a", "甲");
        match boundary {
            "backspace" => engine.note_backspace(),
            "edit" => {
                engine.push('x');
                engine.backspace();
            }
            "private-exit" => {
                engine.set_private(true);
                commit(&mut engine, "ijkl", "丙");
                engine.set_private(false);
            }
            "learning-exit" => {
                engine.set_learning(false);
                commit(&mut engine, "ijkl", "丙");
                engine.set_learning(true);
            }
            _ => unreachable!(),
        }
        commit(&mut engine, "efgh", "乙");
        assert!(!has_word(&mut engine, "abef", "甲乙"), "{boundary}");
        assert!(!has_word(&mut engine, "ijef", "丙乙"), "{boundary}");
    }
}

#[test]
fn wubi_auto_commit_checks_learned_code_collisions() {
    let mut engine = engine();
    commit(&mut engine, "a", "甲");
    commit(&mut engine, "efgh", "乙");
    engine.set_code_rules(true, true);
    engine.set_input("abef");
    assert_eq!(engine.commit_unique_code().as_deref(), Some("甲乙"));
    assert!(!engine.composing_code());
}
