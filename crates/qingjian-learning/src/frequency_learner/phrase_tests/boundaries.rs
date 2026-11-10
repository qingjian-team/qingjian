//! 焦点恢复与同码冲突不绕过组词边界。

use super::{commit, engine, has_word};
use qingjian_core::EngineSession;
use qingjian_dictionary::CodeTable;

#[test]
fn contact_commits_break_the_code_phrase_window() {
    let mut engine = engine();
    let table = CodeTable::parse(
        "甲\tabcd\t10\n乙\tefgh\t10\n丙\tijkl\t10\n丁\tmnop\t10\n甲乙\tabef\t10\n",
    )
    .unwrap();
    let mut scheme = qingjian_core::CodeScheme::new("wubi", table);
    scheme.wubi_phrases = true;
    engine.set_code_scheme(Some(scheme));
    assert_eq!(
        engine.set_contacts(&[qingjian_core::Contact {
            name: "甲乙".into(),
            details: Vec::new(),
        }]),
        1
    );
    commit(&mut engine, "ijkl", "丙");
    commit(&mut engine, "abef", "甲乙");
    commit(&mut engine, "mnop", "丁");
    assert!(!has_word(&mut engine, "aemn", "甲乙丁"));
    assert!(!has_word(&mut engine, "iaem", "丙甲乙丁"));
    assert!(!has_word(&mut engine, "ijmn", "丙丁"));
}

#[test]
fn accepted_cloud_sentence_breaks_the_code_phrase_window() {
    struct Model;
    impl qingjian_core::sentence::LanguageModel for Model {
        fn log_prob(&self, _previous: Option<&str>, word: &str) -> Option<f64> {
            (word == "开").then_some(-1.0)
        }
    }
    let mut engine = engine().with_language_model(Box::new(Model));
    engine.set_phonetic(true);
    commit(&mut engine, "a", "甲");
    engine.set_input("kai");
    assert_eq!(engine.accept_prediction("开"), "开");
    commit(&mut engine, "efgh", "乙");
    assert!(!has_word(&mut engine, "abef", "甲乙"));
}

#[test]
fn reapplying_unchanged_options_keeps_the_phrase_window() {
    let mut engine = engine();
    commit(&mut engine, "a", "甲");
    engine.set_learning(true);
    engine.set_shuangpin_layout(None);
    engine.set_auto_phrase(true, 4);
    commit(&mut engine, "efgh", "乙");
    assert!(has_word(&mut engine, "abef", "甲乙"));
}

#[test]
fn translated_output_breaks_the_code_phrase_window() {
    use qingjian_core::{Language, Sense, Translation};

    let mut engine = engine();
    commit(&mut engine, "a", "甲");
    engine.set_input("efgh");
    let mut candidate = engine.query().unwrap().candidates.items.remove(0);
    candidate.translation = Some(Translation::new(
        Language::English,
        vec![Sense {
            part_of_speech: None,
            text: "B".into(),
            reading: None,
            pronunciation: None,
            fresh: false,
        }],
    ));
    assert_eq!(
        engine.commit_translation(&candidate, 0).as_deref(),
        Some("B")
    );
    commit(&mut engine, "ijkl", "丙");
    assert!(!has_word(&mut engine, "abef", "甲乙"));
    assert!(!has_word(&mut engine, "efij", "乙丙"));
}

#[test]
fn restoring_a_session_does_not_join_words_across_focus_changes() {
    let mut engine = engine();
    commit(&mut engine, "a", "甲");
    let mut session = EngineSession::default();
    engine.swap_session(&mut session);
    engine.set_auto_phrase(false, 4);
    engine.set_auto_phrase(true, 4);
    engine.swap_session(&mut session);
    commit(&mut engine, "efgh", "乙");
    assert!(!has_word(&mut engine, "abef", "甲乙"));
}

#[test]
fn learned_phrases_colliding_with_static_entries_are_not_auto_committed() {
    let mut engine = engine();
    commit(&mut engine, "a", "甲");
    commit(&mut engine, "efgh", "乙");
    let mut scheme = engine.code_scheme().unwrap().clone();
    scheme
        .table
        .merge(CodeTable::parse("别\tabef\t10\n").unwrap());
    scheme.auto_commit = true;
    engine.set_code_scheme(Some(scheme));
    engine.set_input("abef");
    assert_eq!(engine.commit_unique_code(), None);
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items.len(), 2);
}

#[test]
fn schemes_without_phrase_rules_never_learn_new_code_words() {
    let mut engine = engine();
    let mut scheme = engine.code_scheme().unwrap().clone();
    scheme.wubi_phrases = false;
    engine.set_code_scheme(Some(scheme));
    commit(&mut engine, "a", "甲");
    commit(&mut engine, "efgh", "乙");
    assert!(!has_word(&mut engine, "abef", "甲乙"));
}
