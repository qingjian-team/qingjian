//! 自动空格：与上文交界补空格、补的空格记成直通、不知道上文时不补。

use super::*;

fn spaced_engine() -> Engine {
    let mut engine = engine();
    engine.set_auto_space(true);
    engine
}

fn commit_kaifa(engine: &mut Engine) -> String {
    engine.set_input("kaifa");
    let kaifa = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == "开发")
        .unwrap();
    engine.commit(&kaifa)
}

#[test]
fn chinese_after_latin_gets_a_leading_space() {
    let mut engine = spaced_engine();
    engine.note_passthrough('G');
    engine.note_passthrough('o');
    assert_eq!(commit_kaifa(&mut engine), " 开发");
}

#[test]
fn off_by_default() {
    let mut engine = engine();
    engine.note_passthrough('G');
    assert_eq!(commit_kaifa(&mut engine), "开发");
}

#[test]
fn raw_latin_after_chinese_gets_a_leading_space() {
    let mut engine = spaced_engine();
    commit_kaifa(&mut engine);
    engine.set_input("go");
    assert_eq!(engine.take_raw(), " go");
}

#[test]
fn digit_passthrough_after_chinese() {
    let mut engine = spaced_engine();
    commit_kaifa(&mut engine);
    assert!(engine.auto_space_before("3"));
    engine.note_passthrough('3');
    assert!(!engine.auto_space_before("0"));
}

#[test]
fn existing_space_or_punctuation_is_not_doubled() {
    let mut engine = spaced_engine();
    commit_kaifa(&mut engine);
    engine.note_passthrough(' ');
    assert!(!engine.auto_space_before("G"));
    engine.note_passthrough('，');
    assert!(!engine.auto_space_before("G"));
}

#[test]
fn first_commit_uses_application_context() {
    let mut engine = spaced_engine();
    engine.set_rescoring_context(Some("你好".to_owned()));
    engine.set_input("go");
    assert_eq!(engine.take_raw(), " go");
}

#[test]
fn unknown_context_adds_nothing() {
    let mut engine = spaced_engine();
    commit_kaifa(&mut engine);
    engine.break_chain();
    assert!(!engine.auto_space_before("3"));
    engine.set_rescoring_context(None);
    engine.set_input("go");
    assert_eq!(engine.take_raw(), "go");
}

#[test]
fn partly_erased_commit_is_unknown() {
    let mut engine = spaced_engine();
    commit_kaifa(&mut engine);
    engine.note_backspace();
    assert!(!engine.auto_space_before("3"));
}

fn commit_kaifa_translation(engine: &mut Engine) -> Option<String> {
    engine.set_input("kaifa");
    let mut query = engine.query().unwrap();
    engine.annotate(&mut query.candidates);
    let kaifa = query.candidates.items[0].clone();
    engine.commit_translation(&kaifa, 0)
}

#[test]
fn translation_is_spaced_and_becomes_the_context() {
    let mut engine = spaced_engine().with_translator(Box::new(FixedTranslator));
    commit_kaifa(&mut engine);
    assert_eq!(
        commit_kaifa_translation(&mut engine).as_deref(),
        Some(" develop")
    );
    assert!(!engine.auto_space_before("3"));
    assert!(engine.auto_space_before("开"));
}

/// 西文的词与词之间要空格（`hola` 接 `mundo`），自动空格关着也补；汉字后面仍归自动空格管。
#[test]
fn consecutive_translations_are_spaced_without_auto_space() {
    let mut engine = engine().with_translator(Box::new(FixedTranslator));
    assert_eq!(
        commit_kaifa_translation(&mut engine).as_deref(),
        Some("develop")
    );
    assert_eq!(
        commit_kaifa_translation(&mut engine).as_deref(),
        Some(" develop")
    );
    // 退格删掉第二个译词连同补的空格，上文回到第一个译词的末字，下一个照样补
    for _ in 0.." develop".len() {
        engine.note_backspace();
    }
    assert_eq!(
        commit_kaifa_translation(&mut engine).as_deref(),
        Some(" develop")
    );
    commit_kaifa(&mut engine);
    assert_eq!(
        commit_kaifa_translation(&mut engine).as_deref(),
        Some("develop")
    );
}

#[test]
fn custom_phrase_is_committed_verbatim() {
    let mut engine = spaced_engine();
    engine
        .set_custom_phrases(vec![crate::CustomPhrase::new("dk", "用docker部署")])
        .unwrap();
    engine.set_input("dk");
    let phrase = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(engine.commit(&phrase), "用docker部署");
    // 原样上屏的末字照样是上文
    assert!(engine.auto_space_before("3"));
}
