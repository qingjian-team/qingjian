//! AI 候选置顶的协议闭环：异步更新、去重、空格 / 数字上屏与选词期间的稳定性。

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use qingjian_core::{CloudWord, Prediction, PredictionPolicy, PredictionRequest, Predictor};
use qingjian_platform::protocol::{ClientMessage, KeyEvent, KeyOutcome, ServerMessage};
use qingjian_platform::{Config, KeyCombo};
use qingjian_windows_server::Router;

use crate::support::{SESSION, candidate_texts, digit, press, router, type_letters};

#[derive(Clone)]
struct QueuedPredictor {
    automatic: bool,

    sequence: Arc<AtomicU64>,

    submitted: Arc<Mutex<Vec<PredictionRequest>>>,

    replies: Arc<Mutex<VecDeque<Prediction>>>,
}

impl Predictor for QueuedPredictor {
    fn policy(&self) -> PredictionPolicy {
        PredictionPolicy {
            automatic: self.automatic,
            ..PredictionPolicy::default()
        }
    }

    fn submit(&mut self, request: PredictionRequest) {
        self.sequence.store(request.sequence, Ordering::Relaxed);
        self.submitted.lock().unwrap().push(request);
    }

    fn poll(&mut self) -> Option<Prediction> {
        self.replies.lock().unwrap().pop_front()
    }
}

fn with_cloud() -> (Router, QueuedPredictor) {
    let mut router = router();
    let sequence = Arc::new(AtomicU64::new(0));
    let replies = Arc::new(Mutex::new(VecDeque::new()));
    let predictor = QueuedPredictor {
        automatic: true,
        sequence,
        submitted: Arc::new(Mutex::new(Vec::new())),
        replies,
    };
    router
        .engine_mut()
        .set_predictor(Box::new(predictor.clone()));
    (router, predictor)
}

fn prediction(sequence: u64) -> Prediction {
    Prediction {
        sequence,
        words: ["开阀", "开发"]
            .into_iter()
            .map(|text| CloudWord {
                text: text.to_owned(),
                syllables: vec!["kai".into(), "fa".into()],
                reading: None,
            })
            .collect(),
        sentence: None,
    }
}

fn poll(router: &mut Router) -> qingjian_platform::protocol::Frame {
    match router.handle(ClientMessage::Poll { session: SESSION }) {
        Some(ServerMessage::Update { frame, .. }) => frame,
        other => panic!("expected Update, got {other:?}"),
    }
}

#[test]
fn ai_words_are_first_and_space_and_numbers_commit_the_displayed_word() {
    for (key, expected) in [
        (KeyEvent::new(0x20, Some(' '), Default::default()), "开阀"),
        (digit(2), "开发"),
    ] {
        let (mut router, cloud) = with_cloud();
        let (_, _, local) = type_letters(&mut router, "kaifa");
        assert_eq!(candidate_texts(&local).first(), Some(&"开发"));
        cloud
            .replies
            .lock()
            .unwrap()
            .push_back(prediction(cloud.sequence.load(Ordering::Relaxed)));
        let frame = poll(&mut router);
        let texts = candidate_texts(&frame);
        assert_eq!(&texts[..2], ["开阀", "开发"]);
        assert_eq!(texts.iter().filter(|&&text| text == "开发").count(), 1);
        assert_eq!(frame.highlight, 0);
        assert_eq!(press(&mut router, key).1.as_deref(), Some(expected));
        assert!(router.engine_mut().composition().is_empty());
    }
}

#[test]
fn late_ai_result_preserves_the_candidate_the_user_is_selecting() {
    let (mut router, cloud) = with_cloud();
    type_letters(&mut router, "kaifa");
    let (_, _, selected) = press(&mut router, KeyEvent::new(0x28, None, Default::default()));
    let expected = selected.candidates.items[selected.highlight].text.clone();
    cloud
        .replies
        .lock()
        .unwrap()
        .push_back(prediction(cloud.sequence.load(Ordering::Relaxed)));
    let frame = poll(&mut router);
    assert_eq!(frame.candidates, selected.candidates);
    assert_eq!(frame.highlight, selected.highlight);
    assert_eq!(
        press(
            &mut router,
            KeyEvent::new(0x20, Some(' '), Default::default())
        )
        .1
        .as_deref(),
        Some(expected.as_str())
    );
}

#[test]
fn an_old_prediction_does_not_replace_candidates_for_new_input() {
    let (mut router, cloud) = with_cloud();
    type_letters(&mut router, "kaifa");
    let old = cloud.sequence.load(Ordering::Relaxed);
    let (_, _, local) = type_letters(&mut router, "zhe");
    cloud.replies.lock().unwrap().push_back(prediction(old));
    assert_eq!(poll(&mut router).candidates, local.candidates);
}

#[test]
fn ending_composition_in_the_app_cancels_its_prediction() {
    let (mut router, cloud) = with_cloud();
    type_letters(&mut router, "kaifa");
    let old = cloud.sequence.load(Ordering::Relaxed);
    router.handle(ClientMessage::HideCandidates { session: SESSION });
    cloud.replies.lock().unwrap().push_back(prediction(old));
    assert!(
        poll(&mut router)
            .candidates
            .items
            .iter()
            .all(|c| c.text != "开阀")
    );
}

fn trigger(combo: KeyCombo) -> KeyEvent {
    KeyEvent::new(
        combo.key.to_ascii_uppercase() as u32,
        Some(combo.key),
        combo.modifiers.into(),
    )
}

#[test]
fn manual_mode_sends_only_on_shortcut_and_returns_ai_first() {
    let (mut router, mut cloud) = with_cloud();
    cloud.automatic = false;
    router.engine_mut().set_predictor(Box::new(cloud.clone()));
    type_letters(&mut router, "kaifa");
    assert!(cloud.submitted.lock().unwrap().is_empty());
    // 已经翻页，显式请求重新回第一页显示 AI 首选。
    press(&mut router, KeyEvent::new(0x28, None, Default::default()));
    let (outcome, commit, _) = press(&mut router, trigger(KeyCombo::PREDICT_DEFAULT));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(commit.is_none());
    let submitted = cloud.submitted.lock().unwrap();
    assert_eq!(submitted.len(), 1);
    assert!(submitted[0].manual);
    drop(submitted);
    cloud
        .replies
        .lock()
        .unwrap()
        .push_back(prediction(cloud.sequence.load(Ordering::Relaxed)));
    assert_eq!(candidate_texts(&poll(&mut router)).first(), Some(&"开阀"));
    press(&mut router, digit(1));
    assert_eq!(cloud.submitted.lock().unwrap().len(), 1, "选词上屏不请求");
    assert_eq!(
        press(&mut router, trigger(KeyCombo::PREDICT_DEFAULT)).0,
        KeyOutcome::Passthrough
    );
}

#[test]
fn manual_shortcut_is_configurable_and_private_and_english_input_stay_local() {
    let mut config = Config::default();
    config.shortcut.predict = "ctrl+alt+k".parse().unwrap();
    let (_, mut cloud) = with_cloud();
    cloud.automatic = false;
    let mut router = crate::support::router_with((&config).into());
    router.engine_mut().set_predictor(Box::new(cloud.clone()));
    match router.handle(ClientMessage::SyncMode { session: SESSION }) {
        Some(ServerMessage::ModeSync { input, .. }) => {
            assert_eq!(input.predict, Some(config.shortcut.predict));
        }
        other => panic!("expected ModeSync, got {other:?}"),
    }
    type_letters(&mut router, "kaifa");
    assert_eq!(
        press(&mut router, trigger(KeyCombo::PREDICT_DEFAULT)).0,
        KeyOutcome::Passthrough
    );
    press(&mut router, trigger(config.shortcut.predict));
    assert_eq!(cloud.submitted.lock().unwrap().len(), 1);
    let old = cloud.sequence.load(Ordering::Relaxed);
    type_letters(&mut router, "zhe");
    cloud.replies.lock().unwrap().push_back(prediction(old));
    assert!(
        poll(&mut router)
            .candidates
            .items
            .iter()
            .all(|c| c.text != "开阀")
    );
    router.engine_mut().set_private(true);
    press(&mut router, trigger(config.shortcut.predict));
    router.engine_mut().set_private(false);
    router.engine_mut().set_english_mode(true);
    press(&mut router, trigger(config.shortcut.predict));
    assert_eq!(cloud.submitted.lock().unwrap().len(), 1);
}
