//! 私密输入：不学、不记、不发云端；离开后恢复。

use super::{
    CountingLearner, EchoPredictor, FixedTranslator, LearningTranslator, MemoryFiller,
    MemoryLogger, MemoryVocabulary, engine,
};
use crate::Language;
use crate::{CandidateKind, Engine, EngineSession};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

fn pick(engine: &mut Engine, input: &str, text: &str) {
    engine.set_input(input);
    let candidate = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == text && c.kind == CandidateKind::Chinese)
        .unwrap();
    engine.commit(&candidate);
}

#[test]
fn private_input_learns_nothing_and_logs_nothing() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut engine = engine()
        .with_learner(Box::new(CountingLearner(HashMap::new())))
        .with_input_logger(Box::new(MemoryLogger(log.clone())));
    pick(&mut engine, "kaifa", "开放");
    assert_eq!(engine.learner().weight("开放"), 1);
    let logged_before = log.lock().unwrap().len();
    assert!(logged_before > 0);
    let order = |engine: &mut Engine| -> Vec<String> {
        engine.set_input("kaifa");
        let items = engine.query().unwrap().candidates.items;
        engine.clear();
        items.into_iter().map(|c| c.text).collect()
    };
    let ranked = order(&mut engine);

    engine.set_private(true);
    assert!(engine.is_private());
    // 读照常：已学的仍参与排序，私密前后候选顺序一样
    assert_eq!(order(&mut engine), ranked);
    pick(&mut engine, "kaifa", "开发");
    pick(&mut engine, "xian", "先");
    // 删掉重选也不撤销：私密期间没记过
    engine.note_backspace();
    pick(&mut engine, "kaifa", "开放");
    assert_eq!(engine.learner().weight("开发"), 0);
    assert_eq!(engine.learner().weight("先"), 0);
    assert_eq!(engine.learner().weight("开放"), 1);
    assert_eq!(log.lock().unwrap().len(), logged_before);

    engine.set_private(false);
    pick(&mut engine, "kaifa", "开发");
    assert_eq!(engine.learner().weight("开发"), 1);
    assert!(log.lock().unwrap().len() > logged_before);
}

#[test]
fn private_input_sends_nothing_to_the_cloud() {
    let submitted = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let filler = MemoryFiller::default();
    let requested = filler.requested.clone();
    let mut engine = engine()
        .with_predictor(Box::new(EchoPredictor {
            submitted: submitted.clone(),
            replies: Vec::new(),
            sentence: true,
        }))
        .with_translator(Box::new(LearningTranslator::default()))
        .with_gloss_filler(Box::new(filler));
    engine.set_private(true);
    engine.set_input("kaifa");
    let query = engine.query().unwrap();
    assert_eq!(
        engine.request_prediction(None, &query.candidates.items),
        None
    );
    assert_eq!(engine.request_translation("开放"), None);
    // 释义表里没有 开放：平时会问释义兜底，私密中不问
    pick(&mut engine, "kaifa", "开放");
    assert!(submitted.borrow().is_empty());
    assert!(requested.lock().unwrap().is_empty());

    engine.set_private(false);
    engine.set_input("kaifa");
    let query = engine.query().unwrap();
    assert!(
        engine
            .request_prediction(None, &query.candidates.items)
            .is_some()
    );
    pick(&mut engine, "kaifa", "开放");
    assert_eq!(requested.lock().unwrap().as_slice(), ["开放"]);
}

#[test]
fn private_input_does_not_write_vocabulary_and_normal_input_recovers() {
    let book = Arc::new(Mutex::new(HashMap::new()));
    let mut engine = engine()
        .with_translator(Box::new(FixedTranslator))
        .with_vocabulary_tracker(Box::new(MemoryVocabulary(book.clone())));

    engine.set_private(true);
    engine.set_input("kaifa");
    let mut query = engine.query().unwrap();
    engine.annotate(&mut query.candidates);
    let candidate = query.candidates.items[0].clone();
    engine.note_displayed(query.candidates.items.iter());
    assert_eq!(engine.commit(&candidate), "开发");
    assert!(book.lock().unwrap().is_empty());

    engine.set_input("kaifa");
    let mut query = engine.query().unwrap();
    engine.annotate(&mut query.candidates);
    let candidate = query.candidates.items[0].clone();
    engine.note_displayed(query.candidates.items.iter());
    assert_eq!(
        engine.commit_translation(&candidate, 0).as_deref(),
        Some("develop")
    );
    assert!(book.lock().unwrap().is_empty());

    engine.set_private(false);
    engine.set_input("kaifa");
    let mut query = engine.query().unwrap();
    engine.annotate(&mut query.candidates);
    let candidate = query.candidates.items[0].clone();
    engine.note_displayed(query.candidates.items.iter());
    engine.commit(&candidate);
    assert_eq!(
        book.lock()
            .unwrap()
            .get(&(Language::English, "develop".into())),
        Some(&(1, 1, 0))
    );

    engine.set_input("kaifa");
    let mut query = engine.query().unwrap();
    engine.annotate(&mut query.candidates);
    let candidate = query.candidates.items[0].clone();
    engine.note_displayed(query.candidates.items.iter());
    assert_eq!(
        engine.commit_translation(&candidate, 0).as_deref(),
        Some("develop")
    );
    assert_eq!(
        book.lock()
            .unwrap()
            .get(&(Language::English, "develop".into())),
        Some(&(2, 2, 1))
    );
}

#[test]
fn explicit_discard_removes_private_history_and_saved_input_state() {
    let mut engine = engine();
    engine.set_private(true);
    pick(&mut engine, "kaifa", "开发");
    engine.note_passthrough('S');
    engine.set_input("kaifa");
    engine.query().unwrap();
    engine.discard_input();
    engine.set_private(false);
    assert!(engine.history().is_empty());
    assert!(engine.composition().is_empty());
    assert!(engine.passthrough_pending.is_empty());
    assert!(engine.journal.recent_commits.is_empty());
    assert!(engine.chain.previous().is_none());
    assert!(engine.chain.buffer_key().is_empty());
    assert!(engine.caches.last_query.borrow().is_none());

    engine.set_input("private");
    engine.history_mut().record("secret");
    let mut saved = EngineSession::default();
    engine.swap_session(&mut saved);
    saved.discard_input();
    engine.swap_session(&mut saved);
    assert!(engine.composition().is_empty());
    assert!(engine.history().is_empty());
}

#[test]
fn reporting_privacy_after_first_frame_does_not_discard_that_frame() {
    let mut engine = engine();
    engine.set_input("k");
    engine.query().unwrap();
    engine.set_private(true);
    assert_eq!(engine.composition().text(), "k");
    engine.set_private(false);
    assert_eq!(engine.composition().text(), "k");
}
