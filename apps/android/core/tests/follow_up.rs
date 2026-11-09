//! 上屏后的联想（#861）：选词后候选条接着给联想词，点了上屏并接着联想；光标前不再是刚上屏的词、或开关关着就不给。
use std::path::Path;

use qingjian_android::{Frame, InputOptions, Session};
use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_lm::BigramModel;

const UNIGRAM: &str = "<s>\t100\n我们\t50\n一起\t30\n去\t20\n吧\t10\n";
const BIGRAM: &str = "我们\t一起\t25\n我们\t去\t5\n一起\t去\t9\n";

fn session() -> Session {
    let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assets/sample");
    let model = BigramModel::parse(UNIGRAM, BIGRAM).unwrap();
    let engine = Engine::new(Dictionary::from_path(sample.join("dict.tsv")).unwrap())
        .with_language_model(Box::new(model));
    let code_table =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assets/sample/wubi86.tsv");
    let user_dir = std::env::temp_dir().join(format!("qingjian-follow-{}", std::process::id()));
    Session::new(engine, code_table, sample, user_dir)
}

fn texts(frame: &Frame) -> Vec<&str> {
    frame.candidates.iter().map(|c| c.text.as_str()).collect()
}

/// 打 women 选上 我们，返回上屏那一帧。
fn commit_women(session: &mut Session) -> Frame {
    let mut frame = None;
    for c in "women".chars() {
        frame = Some(session.push(c));
    }
    let index = texts(frame.as_ref().unwrap())
        .iter()
        .position(|text| *text == "我们")
        .unwrap();
    let frame = session.select(index);
    assert_eq!(frame.commit, "我们");
    frame
}

#[test]
fn a_committed_word_offers_follow_ups_that_chain() {
    let mut session = session();
    let frame = commit_women(&mut session);
    assert_eq!(texts(&frame), ["一起", "去"]);
    // 光标停下后壳问补全：刚上屏的词还在光标前就留着
    let frame = session.complete("我们");
    assert_eq!(texts(&frame), ["一起", "去"]);
    let frame = session.select(0);
    assert_eq!(frame.commit, "一起");
    assert_eq!(texts(&frame), ["去"]);
    let frame = session.select(0);
    assert_eq!(frame.commit, "去");
    assert!(frame.candidates.is_empty());
}

#[test]
fn follow_ups_go_away_when_the_word_is_no_longer_before_the_cursor() {
    let mut session = session();
    commit_women(&mut session);
    assert!(session.complete("我").candidates.is_empty());
    // 不再接着给：下一次问补全也是空的
    assert!(session.complete("我们").candidates.is_empty());
}

#[test]
fn typing_replaces_follow_ups_with_candidates_and_backspace_is_not_held() {
    let mut session = session();
    commit_women(&mut session);
    assert!(!session.has_offer(), "删除键照常删字");
    let frame = session.push('n');
    assert!(!texts(&frame).contains(&"一起"));
}

#[test]
fn the_setting_turns_follow_ups_off() {
    let mut session = session();
    let options: InputOptions = serde_json::from_str(
        r#"{"full_width_punctuation":true,"english_in_chinese":true,"chinese_first":false,"follow_ups":false}"#,
    )
    .unwrap();
    session.set_options(&options);
    assert!(commit_women(&mut session).candidates.is_empty());
}
