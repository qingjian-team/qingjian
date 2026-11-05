//! 通讯录：名字候选带人头标记，上屏后接着给电话、邮箱，光标前不再是这个名字就收起。
use std::path::Path;

use qingjian_android::Session;
use qingjian_core::{Contact, Engine};
use qingjian_dictionary::Dictionary;

fn session(details: bool) -> Session {
    let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assets/sample");
    let engine = Engine::new(Dictionary::from_path(sample.join("dict.tsv")).unwrap());
    let code_table =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assets/schemes/wubi86/table.tsv");
    let user_dir = std::env::temp_dir().join(format!("qingjian-contacts-{}", std::process::id()));
    let mut session = Session::new(engine, code_table, sample, user_dir);
    let contacts = [Contact {
        name: "曾小贤".into(),
        details: vec!["13800138000".into(), "zxx@example.com".into()],
    }];
    assert_eq!(session.set_contacts(&contacts, details), 1);
    session
}

fn type_keys(session: &mut Session, keys: &str) -> qingjian_android::Frame {
    let mut frame = None;
    for c in keys.chars() {
        frame = Some(session.push(c));
    }
    frame.unwrap()
}

/// 打 zengxiaoxian 选上 曾小贤，返回上屏那一帧。
fn commit_name(session: &mut Session) -> qingjian_android::Frame {
    let frame = type_keys(session, "zengxiaoxian");
    let index = frame
        .candidates
        .iter()
        .position(|c| c.text == "曾小贤")
        .unwrap();
    let frame = session.select(index);
    assert_eq!(frame.commit, "曾小贤");
    frame
}

#[test]
fn a_committed_name_offers_its_details() {
    let mut session = session(true);
    let frame = type_keys(&mut session, "zengxiaoxian");
    let index = frame
        .candidates
        .iter()
        .position(|c| c.text == "曾小贤")
        .expect("name is a candidate");
    assert!(index <= 1, "{:?}", frame.candidates);
    assert!(frame.candidates[index].contact);
    assert!(
        !frame
            .candidates
            .iter()
            .any(|c| c.text != "曾小贤" && c.contact)
    );

    let frame = session.select(index);
    assert_eq!(frame.commit, "曾小贤");
    let offered: Vec<&str> = frame.candidates.iter().map(|c| c.text.as_str()).collect();
    assert_eq!(offered, ["13800138000", "zxx@example.com"]);

    // 光标停下后壳问补全：名字还在光标前就留着
    let frame = session.complete("我是曾小贤");
    assert_eq!(frame.candidates.len(), 2);
    let frame = session.select(0);
    assert_eq!(frame.commit, "13800138000");
    assert!(frame.candidates.is_empty());
}

#[test]
fn details_go_away_when_the_name_is_no_longer_before_the_cursor() {
    let mut session = session(true);
    commit_name(&mut session);
    let frame = session.complete("曾小");
    assert!(frame.candidates.is_empty());

    // 接着打字也收起
    commit_name(&mut session);
    let frame = session.push('n');
    assert!(!frame.candidates.iter().any(|c| c.text == "13800138000"));
}

#[test]
fn details_stay_hidden_when_turned_off() {
    let mut session = session(false);
    let frame = commit_name(&mut session);
    assert!(frame.candidates.is_empty());
}
