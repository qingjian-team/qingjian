//! 通讯录：名字并进查词、整段对得上时进第一页，上屏不学、不记，不发给云端。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::{CountingLearner, EchoPredictor, MemoryLogger};
use crate::shuangpin::Scheme;
use crate::{CandidateKind, Contact, Engine};
use qingjian_dictionary::Dictionary;

const WORDS: &str = "这么\tzhe me\t50000\n怎么\tzen me\t60000\n战网\tzhan wang\t3000\n正文\tzheng wen\t8000\n中文\tzhong wen\t20000\n张\tzhang\t9000\n伟\twei\t5000\n爸爸\tba ba\t30000\n八\tba\t20000\n";

fn contacts() -> Vec<Contact> {
    vec![
        Contact {
            name: "张伟".into(),
            details: vec!["13800138000".into(), "zw@example.com".into()],
        },
        Contact {
            name: "爸爸".into(),
            details: vec!["13900139000".into()],
        },
    ]
}

fn engine() -> Engine {
    let mut engine = Engine::new(Dictionary::parse(WORDS).unwrap());
    assert_eq!(engine.set_contacts(&contacts()), 2);
    engine
}

fn texts(engine: &mut Engine, input: &str) -> Vec<String> {
    engine.set_input(input);
    let items = engine.query().unwrap().candidates.items;
    engine.clear();
    items.into_iter().map(|c| c.text).collect()
}

#[test]
fn whole_input_names_come_right_after_the_first_candidate() {
    let mut engine = engine();
    let abbreviated = texts(&mut engine, "zw");
    assert!(
        abbreviated[..2].contains(&"张伟".to_owned()),
        "{abbreviated:?}"
    );
    let full = texts(&mut engine, "zhangwei");
    assert!(full[..2].contains(&"张伟".to_owned()), "{full:?}");
    engine.set_shuangpin(Some(Scheme::Xiaohe));
    let shuangpin = texts(&mut engine, "vhww");
    assert!(shuangpin[..2].contains(&"张伟".to_owned()), "{shuangpin:?}");
}

#[test]
fn a_name_that_is_also_a_word_shows_once() {
    let mut engine = engine();
    let list = texts(&mut engine, "baba");
    assert_eq!(list.iter().filter(|t| *t == "爸爸").count(), 1);
    assert_eq!(list[0], "爸爸");
    assert!(engine.is_contact("爸爸"));
    assert_eq!(engine.contact_details("爸爸"), ["13900139000"]);
    assert!(engine.contact_details("中文").is_empty());
}

#[test]
fn committing_a_name_learns_and_logs_nothing() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut engine = engine()
        .with_learner(Box::new(CountingLearner(HashMap::new())))
        .with_input_logger(Box::new(MemoryLogger(log.clone())));
    let pick = |engine: &mut Engine, input: &str, text: &str| {
        engine.set_input(input);
        let candidate = engine
            .query()
            .unwrap()
            .candidates
            .items
            .into_iter()
            .find(|c| c.text == text && c.kind == CandidateKind::Chinese)
            .unwrap();
        engine.commit(&candidate)
    };
    assert_eq!(pick(&mut engine, "zhangwei", "张伟"), "张伟");
    assert_eq!(engine.learner().weight("张伟"), 0);
    assert!(log.lock().unwrap().is_empty());
    pick(&mut engine, "zhongwen", "中文");
    assert_eq!(engine.learner().weight("中文"), 1);
    let logged = format!("{:?}", log.lock().unwrap());
    assert!(!logged.contains("张伟"), "{logged}");
}

#[test]
fn names_are_not_sent_to_the_cloud() {
    let submitted = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut engine = engine().with_predictor(Box::new(EchoPredictor {
        submitted: submitted.clone(),
        replies: Vec::new(),
        sentence: true,
    }));
    engine.set_input("zhangwei");
    let query = engine.query().unwrap();
    assert!(query.candidates.items.iter().any(|c| c.text == "张伟"));
    engine.request_prediction(None, &query.candidates.items);
    let request = submitted.borrow()[0].clone();
    assert!(
        !request.candidates.iter().any(|c| c == "张伟"),
        "{request:?}"
    );
    assert!(!request.guess.contains("张伟"), "{request:?}");
}

#[test]
fn clearing_contacts_removes_the_names() {
    let mut engine = engine();
    assert_eq!(engine.set_contacts(&[]), 0);
    assert!(!engine.is_contact("张伟"));
    assert!(engine.contact_details("张伟").is_empty());
    // 简拼凑不出单字拼起来的 张伟，名字词库摘掉后就没了
    assert!(!texts(&mut engine, "zw").contains(&"张伟".to_owned()));
}
