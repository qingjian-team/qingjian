//! Linux Server 云结果回显与私密输入边界；用内存通道代替网络服务。

use qingjian_core::{
    CandidateKind, CloudWord, Engine, Prediction, PredictionKind, PredictionPolicy,
    PredictionRequest, Predictor,
};
use qingjian_dictionary::Dictionary;
use qingjian_linux_server::{Router, RouterConfig};
use qingjian_platform::protocol::{
    ClientMessage, Frame, KeyEvent, KeyModifiers, PROTOCOL_VERSION, ServerMessage, SessionId,
};
use serde_json::json;
use std::sync::mpsc::{self, Receiver, Sender};

struct ChannelPredictor {
    /// 是否支持停键后自动联想；`false` 模拟关掉「自动联想」只留按需触发。
    automatic: bool,

    requests: Sender<PredictionRequest>,

    responses: Receiver<Prediction>,
}

impl Predictor for ChannelPredictor {
    fn policy(&self) -> PredictionPolicy {
        PredictionPolicy {
            automatic: self.automatic,
            slots: 2,
            ..PredictionPolicy::default()
        }
    }

    fn submit(&mut self, request: PredictionRequest) {
        self.requests.send(request).unwrap();
    }

    fn poll(&mut self) -> Option<Prediction> {
        self.responses.try_recv().ok()
    }
}

fn setup() -> (Router, Receiver<PredictionRequest>, Sender<Prediction>) {
    setup_with_automatic(true)
}

fn setup_with_automatic(
    automatic: bool,
) -> (Router, Receiver<PredictionRequest>, Sender<Prediction>) {
    let dictionary = Dictionary::parse("你\tni\t100\n泥\tni\t90\n拟\tni\t80\n逆\tni\t70\n腻\tni\t60\n匿\tni\t50\n溺\tni\t40\n尼\tni\t30\n呢\tni\t20\n").unwrap();
    let (request_tx, requests) = mpsc::channel();
    let (responses, response_rx) = mpsc::channel();
    let engine = Engine::new(dictionary).with_predictor(Box::new(ChannelPredictor {
        automatic,
        requests: request_tx,
        responses: response_rx,
    }));
    let config = RouterConfig {
        cloud_slots: 2,
        cloud_enabled: true,
        ..RouterConfig::default()
    };
    let mut router = Router::new(engine, config);
    router.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: Some("test".into()),
        protocol: PROTOCOL_VERSION,
    });
    router.handle(ClientMessage::Privacy {
        session: SessionId(1),
        private: false,
    });
    (router, requests, responses)
}

fn type_ni(router: &mut Router) {
    for c in ['n', 'i'] {
        router.handle(ClientMessage::Key {
            session: SessionId(1),
            event: KeyEvent::new(c as u32, Some(c), KeyModifiers::default()),
        });
    }
}

fn poll(router: &mut Router) -> Frame {
    match router.handle(ClientMessage::Poll {
        session: SessionId(1),
    }) {
        Some(ServerMessage::Update { frame, .. }) => frame,
        _ => panic!("expected update"),
    }
}

#[test]
fn cloud_word_and_sentence_arrive_after_local_frame() {
    let (mut router, requests, responses) = setup();
    type_ni(&mut router);
    assert_eq!(poll(&mut router).candidates.items[0].text, "你");
    let request = requests.try_iter().last().unwrap();
    responses
        .send(Prediction {
            sequence: request.sequence,
            words: vec![CloudWord {
                text: "妮".into(),
                syllables: vec!["ni".into()],
                reading: None,
            }],
            sentence: Some("你好".into()),
        })
        .unwrap();
    let frame = poll(&mut router);
    assert_eq!(frame.candidates.items[0].text, "你");
    assert!(
        frame
            .candidates
            .items
            .iter()
            .any(|c| c.text == "妮" && c.kind == CandidateKind::Cloud)
    );
    assert_eq!(frame.sentence.as_deref(), Some("你好"));
}

#[test]
fn privacy_change_discards_pending_cloud_reply() {
    let (mut router, requests, responses) = setup();
    type_ni(&mut router);
    let request = requests.try_iter().last().unwrap();
    router.handle(ClientMessage::Privacy {
        session: SessionId(1),
        private: true,
    });
    responses
        .send(Prediction {
            sequence: request.sequence,
            words: vec![CloudWord {
                text: "妮".into(),
                syllables: vec!["ni".into()],
                reading: None,
            }],
            sentence: Some("你好".into()),
        })
        .unwrap();
    assert!(poll(&mut router).is_empty());
    type_ni(&mut router);
    assert!(requests.try_recv().is_err());
    assert!(
        poll(&mut router)
            .candidates
            .items
            .iter()
            .all(|c| c.kind != CandidateKind::Cloud)
    );
}

#[test]
fn question_answer_uses_cloud_candidate_without_pinyin_syllables() {
    let (mut router, requests, responses) = setup();
    // 本地拆不出字的问题：umumumu 本地就拆出 森，云端同字的答案会被去重
    for c in "usangemu".chars() {
        router.handle(ClientMessage::Key {
            session: SessionId(1),
            event: KeyEvent::new(c as u32, Some(c), KeyModifiers::default()),
        });
    }
    let request = requests.try_iter().last().unwrap();
    assert_eq!(request.kind, PredictionKind::Question);
    responses
        .send(Prediction {
            sequence: request.sequence,
            words: vec![CloudWord {
                text: "森".into(),
                syllables: Vec::new(),
                reading: Some("sēn".into()),
            }],
            sentence: None,
        })
        .unwrap();
    let frame = poll(&mut router);
    assert!(
        frame
            .candidates
            .items
            .iter()
            .any(|candidate| candidate.text == "森" && candidate.kind == CandidateKind::Cloud)
    );
}

#[test]
fn cloud_reply_does_not_replace_the_highlighted_local_candidate() {
    let (mut router, requests, responses) = setup();
    type_ni(&mut router);
    let request = requests.try_iter().last().unwrap();
    for _ in 0..8 {
        router.handle(ClientMessage::Key {
            session: SessionId(1),
            event: KeyEvent::new(0x28, None, KeyModifiers::default()),
        });
    }
    let before = poll(&mut router);
    assert_eq!(before.highlight, 8);
    let selected = before.candidates.items[8].text.clone();
    responses
        .send(Prediction {
            sequence: request.sequence,
            words: vec![CloudWord {
                text: "妮".into(),
                syllables: vec!["ni".into()],
                reading: None,
            }],
            sentence: Some("你好".into()),
        })
        .unwrap();
    let frame = poll(&mut router);
    assert_eq!(frame.candidates.items[8].text, selected);
    assert!(
        frame
            .candidates
            .items
            .iter()
            .all(|candidate| candidate.kind != CandidateKind::Cloud)
    );
    assert_eq!(frame.sentence.as_deref(), Some("你好"));
}

#[test]
fn linux_key_passes_application_context_and_ignores_it_in_sensitive_fields() {
    let (mut router, requests, _) = setup();
    let caps = |router: &mut Router, sensitive| {
        router.handle_linux(
            json!({"LinuxEvent": {"session": 1, "event": {"Capabilities": {
                "sensitive": sensitive, "password": false, "disabled": false
            }}}}),
        );
    };
    let key = |router: &mut Router, character, surrounding: serde_json::Value| {
        router.handle_linux(json!({"LinuxEvent": {"session": 1, "event": {"Key": {
            "event": KeyEvent::new(character as u32, Some(character), KeyModifiers::default()),
            "release": false, "surrounding": surrounding
        }}}}));
    };
    caps(&mut router, false);
    key(
        &mut router,
        'n',
        json!({"before": "你好😀", "after": "世界"}),
    );
    key(
        &mut router,
        'i',
        json!({"before": "你好😀", "after": "世界"}),
    );
    let request = requests.try_iter().last().unwrap();
    assert_eq!(request.before, "你好😀");
    assert_eq!(request.after, "世界");

    caps(&mut router, true);
    key(&mut router, 'n', json!({"before": "秘密", "after": "内容"}));
    key(&mut router, 'i', json!({"before": "秘密", "after": "内容"}));
    assert!(requests.try_recv().is_err());
}

/// 走插件那条入口（`LinuxEvent::Key`）按一个键：手动联想的组合键只在这条路上拦。
fn press_key(router: &mut Router, event: KeyEvent) {
    router.handle_linux(json!({"LinuxEvent": {"session": 1, "event": {"Key": {
        "event": event, "release": false, "surrounding": serde_json::Value::Null
    }}}}));
}

/// `Capabilities` 握手：`LinuxEvent::Key` 在这之前整键放行。
fn enable_capabilities(router: &mut Router) {
    router.handle_linux(
        json!({"LinuxEvent": {"session": 1, "event": {"Capabilities": {
            "sensitive": false, "password": false, "disabled": false
        }}}}),
    );
}

/// 缺省的手动联想组合键（`[shortcut] predict`，Ctrl + Alt + J）。
fn predict_combo_key() -> KeyEvent {
    let combo = RouterConfig::default().predict;
    KeyEvent::new(
        combo.key as u32,
        Some(combo.key),
        KeyModifiers::from(combo.modifiers),
    )
}

fn reply_with(responses: &Sender<Prediction>, sequence: u64, text: &str) {
    responses
        .send(Prediction {
            sequence,
            words: vec![CloudWord {
                text: text.into(),
                syllables: vec!["ni".into()],
                reading: None,
            }],
            sentence: Some("你好".into()),
        })
        .unwrap();
}

#[test]
fn manual_combo_puts_cloud_words_first() {
    let (mut router, requests, responses) = setup();
    enable_capabilities(&mut router);
    press_key(
        &mut router,
        KeyEvent::new('n' as u32, Some('n'), KeyModifiers::default()),
    );
    press_key(
        &mut router,
        KeyEvent::new('i' as u32, Some('i'), KeyModifiers::default()),
    );
    assert!(requests.try_recv().is_ok(), "停键后的自动请求先出去");
    press_key(&mut router, predict_combo_key());
    let request = requests.try_recv().expect("组合键要发出按需请求");
    assert!(request.manual);
    reply_with(&responses, request.sequence, "妮");
    let frame = poll(&mut router);
    assert_eq!(frame.candidates.items[0].text, "妮");
    assert_eq!(frame.candidates.items[0].kind, CandidateKind::Cloud);
    assert_eq!(frame.candidates.items[1].text, "你");
    assert_eq!(frame.sentence.as_deref(), Some("你好"));
}

#[test]
fn manual_prediction_works_without_automatic() {
    let (mut router, requests, responses) = setup_with_automatic(false);
    enable_capabilities(&mut router);
    press_key(
        &mut router,
        KeyEvent::new('n' as u32, Some('n'), KeyModifiers::default()),
    );
    press_key(
        &mut router,
        KeyEvent::new('i' as u32, Some('i'), KeyModifiers::default()),
    );
    poll(&mut router);
    assert!(requests.try_recv().is_err(), "关掉自动联想就不该有请求");
    press_key(&mut router, predict_combo_key());
    let request = requests.try_recv().expect("按需联想不受自动开关影响");
    assert!(request.manual);
    reply_with(&responses, request.sequence, "妮");
    let frame = poll(&mut router);
    assert_eq!(frame.candidates.items[0].text, "妮");
    assert_eq!(frame.candidates.items[0].kind, CandidateKind::Cloud);
}

#[test]
fn late_manual_result_does_not_move_the_highlight() {
    let (mut router, requests, responses) = setup();
    enable_capabilities(&mut router);
    press_key(
        &mut router,
        KeyEvent::new('n' as u32, Some('n'), KeyModifiers::default()),
    );
    press_key(
        &mut router,
        KeyEvent::new('i' as u32, Some('i'), KeyModifiers::default()),
    );
    assert!(requests.try_recv().is_ok());
    press_key(&mut router, predict_combo_key());
    let request = requests.try_recv().unwrap();
    press_key(
        &mut router,
        KeyEvent::new(0x28, None, KeyModifiers::default()),
    );
    reply_with(&responses, request.sequence, "妮");
    let frame = poll(&mut router);
    assert!(
        frame
            .candidates
            .items
            .iter()
            .all(|candidate| candidate.kind != CandidateKind::Cloud),
        "移动过高亮后晚到的按需结果不再插词"
    );
    assert_eq!(frame.sentence.as_deref(), Some("你好"));
}

#[test]
fn predict_combo_passes_through_outside_composition() {
    let (mut router, requests, _) = setup();
    enable_capabilities(&mut router);
    press_key(&mut router, predict_combo_key());
    assert!(requests.try_recv().is_err(), "没在组句时组合键归应用");
}
