//! 自动末尾、手动置顶的协议闭环：异步更新、去重、空格 / 数字上屏与选词期间的稳定性。

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use qingjian_core::{CloudWord, Prediction, PredictionPolicy, PredictionRequest, Predictor};
use qingjian_platform::protocol::{ClientMessage, KeyEvent, KeyOutcome, ServerMessage};
use qingjian_platform::{Config, KeyCombo};
use qingjian_windows_server::Router;

use crate::support::{
    Prefers, RouterConfig, SESSION, candidate_texts, digit, press, router_with, type_letters,
};

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
    let mut router = router_with(RouterConfig {
        cloud_enabled: true,
        ..RouterConfig::default()
    });
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
        press(&mut router, trigger(KeyCombo::PREDICT_DEFAULT));
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
fn local_rescoring_keeps_ai_candidates_at_the_front() {
    let (mut router, cloud) = with_cloud();
    router
        .engine_mut()
        .set_async_sentence_scorer(Some(Box::new(Prefers("你它"))));
    type_letters(&mut router, "nita");
    press(&mut router, trigger(KeyCombo::PREDICT_DEFAULT));
    cloud.replies.lock().unwrap().push_back(Prediction {
        sequence: cloud.sequence.load(Ordering::Relaxed),
        words: vec![CloudWord {
            text: "你她".to_owned(),
            syllables: vec!["ni".into(), "ta".into()],
            reading: None,
        }],
        sentence: None,
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        let frame = poll(&mut router);
        let texts = candidate_texts(&frame);
        assert_eq!(texts.first(), Some(&"你她"));
        if texts.get(1) == Some(&"你它") {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "模型重排未完成：{texts:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
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
fn period_prediction_shortcut_uses_the_windows_period_key() {
    let mut config = Config::default();
    config.predict.enabled = true;
    config.shortcut.predict = "ctrl+alt+.".parse().unwrap();
    let (_, mut cloud) = with_cloud();
    cloud.automatic = false;
    let mut router = crate::support::router_with((&config).into());
    router.engine_mut().set_predictor(Box::new(cloud.clone()));
    type_letters(&mut router, "kaifa");
    let event = KeyEvent::new(0xBE, Some('.'), config.shortcut.predict.modifiers.into());
    assert_eq!(press(&mut router, event).0, KeyOutcome::Consumed);
    assert!(cloud.submitted.lock().unwrap()[0].manual);
    cloud
        .replies
        .lock()
        .unwrap()
        .push_back(prediction(cloud.sequence.load(Ordering::Relaxed)));
    assert_eq!(candidate_texts(&poll(&mut router)).first(), Some(&"开阀"));
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
    config.predict.enabled = true;
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

#[test]
fn automatic_results_keep_local_first_and_tail_numbering() {
    for choose_cloud in [false, true] {
        let (mut router, cloud) = with_cloud();
        let (_, _, local) = type_letters(&mut router, "kaifa");
        assert!(!cloud.submitted.lock().unwrap().last().unwrap().manual);
        cloud
            .replies
            .lock()
            .unwrap()
            .push_back(prediction(cloud.sequence.load(Ordering::Relaxed)));
        let frame = poll(&mut router);
        let texts = candidate_texts(&frame);
        assert_eq!(texts.first(), candidate_texts(&local).first());
        assert_eq!(texts.last(), Some(&"开阀"));
        assert_eq!(texts.iter().filter(|&&word| word == "开发").count(), 1);
        let key = if choose_cloud {
            digit(texts.len() as u32)
        } else {
            KeyEvent::new(0x20, Some(' '), Default::default())
        };
        let expected = if choose_cloud { "开阀" } else { "开发" };
        assert_eq!(press(&mut router, key).1.as_deref(), Some(expected));
    }
}

#[test]
fn manual_result_preserves_highlight_after_navigation() {
    let (mut router, cloud) = with_cloud();
    type_letters(&mut router, "kaifa");
    press(&mut router, trigger(KeyCombo::PREDICT_DEFAULT));
    let (_, _, selected) = press(&mut router, KeyEvent::new(0x28, None, Default::default()));
    cloud
        .replies
        .lock()
        .unwrap()
        .push_back(prediction(cloud.sequence.load(Ordering::Relaxed)));
    assert_eq!(poll(&mut router).candidates, selected.candidates);
}

#[test]
fn manual_context_refresh_tracks_new_sequence_and_stops_after_the_result() {
    let (mut router, mut cloud) = with_cloud();
    cloud.automatic = false;
    router.engine_mut().set_predictor(Box::new(cloud.clone()));
    type_letters(&mut router, "kaifa");
    router.handle(ClientMessage::Surrounding {
        session: SESSION,
        text: "我需要".into(),
    });
    assert!(cloud.submitted.lock().unwrap().is_empty());
    press(&mut router, trigger(KeyCombo::PREDICT_DEFAULT));
    assert_eq!(cloud.submitted.lock().unwrap()[0].before, "我需要");
    let old = cloud.sequence.load(Ordering::Relaxed);
    router.handle(ClientMessage::Surrounding {
        session: SESSION,
        text: "我还需要".into(),
    });
    let new = cloud.sequence.load(Ordering::Relaxed);
    assert_ne!(old, new);
    assert!(
        cloud
            .submitted
            .lock()
            .unwrap()
            .iter()
            .all(|request| request.manual)
    );
    cloud.replies.lock().unwrap().push_back(prediction(old));
    assert_ne!(candidate_texts(&poll(&mut router)).first(), Some(&"开阀"));
    cloud.replies.lock().unwrap().push_back(prediction(new));
    assert_eq!(candidate_texts(&poll(&mut router)).first(), Some(&"开阀"));
    router.handle(ClientMessage::Surrounding {
        session: SESSION,
        text: "新前文".into(),
    });
    assert_eq!(cloud.submitted.lock().unwrap().len(), 2);
    type_letters(&mut router, "zhe");
    assert_eq!(cloud.submitted.lock().unwrap().len(), 2);
}

#[test]
fn next_input_restores_automatic_placement_after_a_manual_result() {
    let (mut router, cloud) = with_cloud();
    type_letters(&mut router, "kaifa");
    press(&mut router, trigger(KeyCombo::PREDICT_DEFAULT));
    cloud
        .replies
        .lock()
        .unwrap()
        .push_back(prediction(cloud.sequence.load(Ordering::Relaxed)));
    assert_eq!(candidate_texts(&poll(&mut router)).first(), Some(&"开阀"));
    press(&mut router, KeyEvent::new(0x1B, None, Default::default()));
    let (_, _, local) = type_letters(&mut router, "kaifa");
    assert!(!cloud.submitted.lock().unwrap().last().unwrap().manual);
    cloud
        .replies
        .lock()
        .unwrap()
        .push_back(prediction(cloud.sequence.load(Ordering::Relaxed)));
    let frame = poll(&mut router);
    assert_eq!(
        candidate_texts(&frame).first(),
        candidate_texts(&local).first()
    );
    assert_eq!(candidate_texts(&frame).last(), Some(&"开阀"));
}

#[test]
fn navigation_keeps_candidate_cells_but_accepts_late_sentence_completion() {
    for manual in [false, true] {
        for virtual_key in [0x28, 0x22] {
            let (_, cloud) = with_cloud();
            let mut router = router_with(RouterConfig {
                cloud_enabled: true,
                page_size: 1,
                ..RouterConfig::default()
            });
            router.engine_mut().set_predictor(Box::new(cloud.clone()));
            type_letters(&mut router, "kaifa");
            if manual {
                press(&mut router, trigger(KeyCombo::PREDICT_DEFAULT));
            }
            let (_, _, selected) = press(
                &mut router,
                KeyEvent::new(virtual_key, None, Default::default()),
            );
            assert!(selected.page > 0, "应实际离开第一页");
            let mut reply = prediction(cloud.sequence.load(Ordering::Relaxed));
            reply.sentence = Some("开发一个新功能".into());
            cloud.replies.lock().unwrap().push_back(reply);
            let frame = poll(&mut router);
            assert_eq!(frame.candidates, selected.candidates);
            assert_eq!(frame.highlight, selected.highlight);
            assert_eq!(frame.page, selected.page);
            assert_eq!(frame.sentence.as_deref(), Some("开发一个新功能"));
            let (_, committed, _) =
                press(&mut router, KeyEvent::new(0x09, None, Default::default()));
            assert_eq!(committed.as_deref(), Some("开发一个新功能"));
        }
    }
}

#[test]
fn sentence_translation_alone_does_not_register_or_trigger_manual_prediction() {
    let (_, cloud) = with_cloud();
    let mut router = router_with(RouterConfig {
        cloud_enabled: false,
        sentence_translation: true,
        ..RouterConfig::default()
    });
    router.engine_mut().set_predictor(Box::new(cloud.clone()));
    match router.handle(ClientMessage::SyncMode { session: SESSION }) {
        Some(ServerMessage::ModeSync { input, .. }) => assert!(input.predict.is_none()),
        other => panic!("expected ModeSync, got {other:?}"),
    }
    type_letters(&mut router, "kaifa");
    assert_eq!(
        press(&mut router, trigger(KeyCombo::PREDICT_DEFAULT)).0,
        KeyOutcome::Passthrough
    );
    assert!(cloud.submitted.lock().unwrap().is_empty());
}
