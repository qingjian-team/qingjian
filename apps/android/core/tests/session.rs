//! 会话层端到端：样例词库上走一遍 敲键 → 选词 → 上屏。
use std::path::Path;

use qingjian_android::{Frame, InputOptions, Session};
use qingjian_core::{Engine, Language};
use qingjian_dictionary::Dictionary;
use qingjian_translate::Glossary;

fn sample(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../assets/sample")
        .join(name)
}

fn session() -> Session {
    let glossary = Glossary::from_path(Language::English, sample("glossary-en.tsv")).unwrap();
    let engine = Engine::new(Dictionary::from_path(sample("dict.tsv")).unwrap())
        .with_translator(Box::new(glossary));
    let code_table =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assets/schemes/wubi86/table.tsv");
    let user_dir = std::env::temp_dir().join(format!("qingjian-session-{}", std::process::id()));
    Session::new(engine, code_table, sample(""), user_dir)
}

fn type_keys(session: &mut Session, keys: &str) {
    for c in keys.chars() {
        session.push(c);
    }
}

#[test]
fn selecting_first_candidate_commits_and_clears() {
    let mut session = session();
    type_keys(&mut session, "niha");
    let frame = session.push('o');
    assert_eq!(frame.preedit, "ni'hao");
    assert_eq!(frame.candidates[0].text, "你好");

    let frame = session.select(0);
    assert_eq!(frame.commit, "你好");
    assert!(frame.preedit.is_empty());
    assert!(frame.candidates.is_empty());
}

#[test]
fn commit_raw_returns_typed_letters() {
    let mut session = session();
    type_keys(&mut session, "niha");
    let frame = session.commit_raw();
    assert_eq!(frame.commit, "niha");
    assert!(frame.preedit.is_empty());
}

#[test]
fn backspace_to_empty_gives_empty_frame() {
    let mut session = session();
    session.push('n');
    let frame = session.backspace();
    assert!(frame.preedit.is_empty());
    assert!(frame.candidates.is_empty());
    assert!(frame.commit.is_empty());
}

#[test]
fn frame_json_shape() {
    let mut session = session();
    type_keys(&mut session, "niha");
    let json = session.push('o').to_json();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["preedit"], "ni'hao");
    assert_eq!(value["candidates"][0]["text"], "你好");
}

#[test]
fn switching_to_english_commits_typed_letters() {
    let mut session = session();
    type_keys(&mut session, "niha");
    let frame = session.set_english_mode(true);
    assert_eq!(frame.commit, "niha");
    assert!(frame.preedit.is_empty());
}

#[test]
fn selecting_translation_commits_the_gloss() {
    let mut session = session();
    type_keys(&mut session, "kaif");
    let frame = session.push('a');
    let index = frame
        .candidates
        .iter()
        .position(|candidate| candidate.text == "开发")
        .unwrap();
    let gloss = frame.candidates[index].senses[0].text.clone();

    let frame = session.select_translation(index, 0);
    assert_eq!(frame.commit, gloss);
    assert!(frame.preedit.is_empty());
}

#[test]
fn missing_sense_leaves_composition_alone() {
    let mut session = session();
    type_keys(&mut session, "kaif");
    session.push('a');
    let frame = session.select_translation(0, 9);
    assert!(frame.commit.is_empty());
    assert_eq!(frame.preedit, "kai'fa");
}

#[test]
fn forgetting_reports_a_notice() {
    let mut session = session();
    type_keys(&mut session, "kaif");
    session.push('a');
    let frame = session.forget(0);
    assert!(frame.notice.contains("开发"));
    assert_eq!(frame.preedit, "kai'fa");
}

#[test]
fn nine_key_digits_compose_and_pin() {
    let mut session = session();
    session.set_nine_key(true);
    type_keys(&mut session, "5243");
    let frame = session.push('2');
    assert_eq!(frame.preedit, "kai'fa");
    assert_eq!(frame.candidates[0].text, "开发");
    assert_eq!(frame.pinyin_choices[0], "kai");

    let frame = session.pin("kai");
    assert_eq!(frame.candidates[0].text, "开发");
    let frame = session.select(0);
    assert_eq!(frame.commit, "开发");
    assert!(frame.preedit.is_empty());
}

#[test]
fn shuangpin_keys_decode_with_the_chosen_scheme() {
    let mut session = session();
    session.set_shuangpin("xiaohe", false);
    type_keys(&mut session, "nihc");
    let frame = session.select(0);
    assert_eq!(frame.commit, "你好");
    session.set_shuangpin("", false);
    type_keys(&mut session, "nihao");
    assert_eq!(session.select(0).commit, "你好");
}

#[test]
fn wubi_codes_look_up_the_code_table() {
    let mut session = session();
    session.set_wubi(true, false);
    type_keys(&mut session, "wqvb");
    assert_eq!(session.select(0).commit, "你好");
    session.set_wubi(true, true);
    type_keys(&mut session, "nihao");
    assert_eq!(session.select(0).commit, "你好");
    session.set_wubi(false, false);
    type_keys(&mut session, "nihao");
    assert_eq!(session.select(0).commit, "你好");
}

#[test]
fn learning_language_off_drops_translations() {
    let mut session = session();
    let options = |language: &str| -> InputOptions {
        serde_json::from_str(&format!(
            r#"{{"full_width_punctuation":true,"english_in_chinese":true,"chinese_first":false,"learning_language":"{language}"}}"#
        ))
        .unwrap()
    };
    session.set_options(&options("off"));
    type_keys(&mut session, "kaifa");
    let frame = session.set_options(&options("off"));
    assert!(frame.candidates.iter().all(|c| c.senses.is_empty()));

    let frame = session.set_options(&options("en"));
    let kaifa = frame.candidates.iter().find(|c| c.text == "开发").unwrap();
    assert!(!kaifa.senses.is_empty());
}

#[test]
fn translation_engine_loads_without_learning_data_and_needs_cloud() {
    let user_dir =
        std::env::temp_dir().join(format!("qingjian-translation-{}", std::process::id()));
    let engine =
        qingjian_android::assemble_translation(&sample(""), &user_dir, Some(Language::English))
            .unwrap();
    assert_eq!(engine.learning_language(), Language::English);
    let mut session = Session::new(engine, sample("wubi86.tsv"), sample(""), user_dir);
    // 没接云端时不发
    assert!(!session.translate("hello"));
}

fn select_text(session: &mut Session, frame: &Frame, text: &str) -> Frame {
    let index = frame
        .candidates
        .iter()
        .position(|c| c.text == text)
        .unwrap();
    session.select(index)
}

#[test]
fn partial_selection_stays_in_the_composition() {
    let mut session = session();
    type_keys(&mut session, "nihaokaif");
    let frame = session.push('a');
    let frame = select_text(&mut session, &frame, "你好");
    assert_eq!(frame.commit, "");
    assert!(frame.preedit.starts_with("你好"));
    // 空格接着选剩下的，整段选完一起上屏
    let frame = session.space();
    assert_eq!(frame.commit, "你好开发");
    assert!(frame.preedit.is_empty());
}

#[test]
fn backspace_undoes_and_enter_keeps_held_words() {
    let mut session = session();
    type_keys(&mut session, "nihaokaif");
    let frame = session.push('a');
    let frame = select_text(&mut session, &frame, "你好");
    assert!(frame.preedit.starts_with("你好"));
    let frame = session.backspace();
    assert!(!frame.preedit.contains("你好"));
    assert!(frame.candidates.iter().any(|c| c.text == "你好"));

    let frame = select_text(&mut session, &frame, "你好");
    assert_eq!(frame.commit, "");
    assert_eq!(session.commit_raw().commit, "你好kaifa");
}

#[test]
fn holding_selections_can_be_turned_off() {
    let mut session = session();
    let options: InputOptions = serde_json::from_str(
        r#"{"full_width_punctuation":true,"english_in_chinese":true,"chinese_first":false,"hold_selections":false}"#,
    )
    .unwrap();
    session.set_options(&options);
    type_keys(&mut session, "nihaokaif");
    let frame = session.push('a');
    let frame = select_text(&mut session, &frame, "你好");
    assert_eq!(frame.commit, "你好");
}
