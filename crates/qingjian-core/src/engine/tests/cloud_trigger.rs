//! 自动开关、显式触发、私密输入的待发取消与九宫格的请求拼音回归。

use std::sync::{Arc, Mutex};

use qingjian_dictionary::Dictionary;

use super::SAMPLE;
use crate::engine::{Engine, Prediction, PredictionPolicy, PredictionRequest, Predictor};

#[derive(Default)]
struct Requests {
    submitted: Vec<PredictionRequest>,

    cancellations: usize,
}

struct ManualPredictor(Arc<Mutex<Requests>>);

impl Predictor for ManualPredictor {
    fn policy(&self) -> PredictionPolicy {
        PredictionPolicy {
            automatic: false,
            ..PredictionPolicy::default()
        }
    }

    fn submit(&mut self, request: PredictionRequest) {
        self.0.lock().unwrap().submitted.push(request);
    }

    fn cancel(&mut self) {
        self.0.lock().unwrap().cancellations += 1;
    }

    fn poll(&mut self) -> Option<Prediction> {
        None
    }
}

#[test]
fn manual_mode_cancels_automatic_requests_and_allows_explicit_requests() {
    let requests = Arc::new(Mutex::new(Requests::default()));
    let mut engine = Engine::new(Dictionary::parse(SAMPLE).unwrap())
        .with_predictor(Box::new(ManualPredictor(requests.clone())));
    engine.set_input("kaifa");
    let before = requests.lock().unwrap().cancellations;
    assert_eq!(engine.request_prediction(None, &[]), None);
    assert_eq!(requests.lock().unwrap().cancellations, before + 1);
    assert!(requests.lock().unwrap().submitted.is_empty());
    assert!(engine.request_manual_prediction(None, &[]).is_some());
    assert!(requests.lock().unwrap().submitted[0].manual);
    assert!(engine.request_translation("开发").is_some());
    assert!(requests.lock().unwrap().submitted[1].manual);

    let before = requests.lock().unwrap().cancellations;
    engine.set_private(true);
    assert_eq!(requests.lock().unwrap().cancellations, before + 1);
    assert_eq!(engine.request_manual_prediction(None, &[]), None);
    assert_eq!(requests.lock().unwrap().submitted.len(), 2);
    engine.set_private(false);
    engine.discard_input();
    assert!(requests.lock().unwrap().cancellations > before + 1);
}

struct AutomaticPredictor(Arc<Mutex<Requests>>);

impl Predictor for AutomaticPredictor {
    fn policy(&self) -> PredictionPolicy {
        PredictionPolicy::default()
    }

    fn submit(&mut self, request: PredictionRequest) {
        self.0.lock().unwrap().submitted.push(request);
    }

    fn cancel(&mut self) {
        self.0.lock().unwrap().cancellations += 1;
    }

    fn poll(&mut self) -> Option<Prediction> {
        None
    }
}

#[test]
fn nine_key_asks_with_the_pinyin_line() {
    let requests = Arc::new(Mutex::new(Requests::default()));
    let mut engine = Engine::new(Dictionary::parse(SAMPLE).unwrap())
        .with_predictor(Box::new(AutomaticPredictor(requests.clone())));
    engine.set_nine_key(true);
    engine.set_input("52432");
    // 没查过这段数字不知道拼音，不发
    assert_eq!(engine.request_prediction(None, &[]), None);
    engine.query().unwrap();
    assert!(engine.request_prediction(None, &[]).is_some());
    let submitted = &requests.lock().unwrap().submitted;
    assert_eq!(submitted[0].letters, "kaifa");
    assert_eq!(submitted[0].syllables, 2);
}

#[test]
fn local_selection_translation_does_not_enable_cloud_prediction() {
    let requests = Arc::new(Mutex::new(Requests::default()));
    let mut engine = Engine::new(Dictionary::parse(SAMPLE).unwrap());
    engine.set_translation_predictor(Some(Box::new(ManualPredictor(requests.clone()))));
    assert!(!engine.prediction_enabled());
    assert!(engine.translation_enabled());
    engine.set_input("kaifa");
    assert!(engine.request_prediction(None, &[]).is_none());
    assert!(engine.request_manual_prediction(None, &[]).is_none());
    assert!(requests.lock().unwrap().submitted.is_empty());
    assert!(engine.request_translation("开发离线翻译").is_some());
    let sent = &requests.lock().unwrap().submitted;
    assert_eq!(sent.len(), 1);
    assert!(sent[0].manual);
    assert!(sent[0].before.is_empty());
    assert!(sent[0].after.is_empty());
}

#[test]
fn local_translation_respects_private_fields_and_disabling() {
    let requests = Arc::new(Mutex::new(Requests::default()));
    let mut engine = Engine::new(Dictionary::parse(SAMPLE).unwrap());
    engine.set_translation_predictor(Some(Box::new(ManualPredictor(requests.clone()))));
    engine.set_private(true);
    assert!(engine.request_translation("私人内容").is_none());
    assert!(requests.lock().unwrap().submitted.is_empty());
    engine.set_private(false);
    engine.set_translation_predictor(None);
    assert!(!engine.translation_enabled());
    assert!(engine.request_translation("开发").is_none());
}

#[test]
fn manual_translation_preserves_selection_whitespace() {
    let requests = Arc::new(Mutex::new(Requests::default()));
    let mut engine = Engine::new(Dictionary::parse(SAMPLE).unwrap());
    engine.set_translation_predictor(Some(Box::new(ManualPredictor(requests.clone()))));
    assert!(engine.request_translation(" \r\n你好。\r\n ").is_some());
    assert_eq!(
        requests.lock().unwrap().submitted[0].text,
        " \r\n你好。\r\n "
    );
    assert!(engine.request_translation(" \r\n ").is_none());
}

#[test]
fn unavailable_local_translation_never_falls_back_to_cloud() {
    let cloud = Arc::new(Mutex::new(Requests::default()));
    let mut engine = Engine::new(Dictionary::parse(SAMPLE).unwrap())
        .with_predictor(Box::new(ManualPredictor(cloud.clone())));
    engine.set_translation_predictor(Some(Box::new(super::super::NoPredictor)));
    assert!(!engine.translation_enabled());
    assert!(engine.request_translation("本机内容").is_none());
    assert!(cloud.lock().unwrap().submitted.is_empty());
    engine.set_translation_predictor(None);
    assert!(engine.request_translation("明确恢复原提供方").is_some());
    assert_eq!(cloud.lock().unwrap().submitted.len(), 1);
}

struct TranslationReplies {
    replies: std::collections::VecDeque<Prediction>,
}

impl Predictor for TranslationReplies {
    fn policy(&self) -> PredictionPolicy {
        PredictionPolicy::default()
    }
    fn submit(&mut self, request: PredictionRequest) {
        self.replies.push_back(Prediction {
            sequence: request.sequence,
            words: Vec::new(),
            sentence: Some(request.text),
        });
    }
    fn poll(&mut self) -> Option<Prediction> {
        self.replies.pop_front()
    }
}

#[test]
fn cancelled_local_translation_cannot_replace_new_selection() {
    let mut engine = Engine::new(Dictionary::parse(SAMPLE).unwrap());
    engine.set_translation_predictor(Some(Box::new(TranslationReplies {
        replies: std::collections::VecDeque::new(),
    })));
    engine.request_translation("旧选区");
    engine.cancel_prediction();
    let sequence = engine.request_translation("新选区").unwrap();
    let reply = engine.poll_prediction().unwrap();
    assert_eq!(reply.sequence, sequence);
    assert_eq!(reply.sentence.as_deref(), Some("新选区"));
    assert!(engine.poll_prediction().is_none());
}
