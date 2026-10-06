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
    requests: Sender<PredictionRequest>,

    responses: Receiver<Prediction>,
}

impl Predictor for ChannelPredictor {
    fn policy(&self) -> PredictionPolicy {
        PredictionPolicy {
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
    let dictionary = Dictionary::parse("你\tni\t100\n泥\tni\t90\n拟\tni\t80\n逆\tni\t70\n腻\tni\t60\n匿\tni\t50\n溺\tni\t40\n尼\tni\t30\n呢\tni\t20\n").unwrap();
    let (request_tx, requests) = mpsc::channel();
    let (responses, response_rx) = mpsc::channel();
    let engine = Engine::new(dictionary).with_predictor(Box::new(ChannelPredictor {
        requests: request_tx,
        responses: response_rx,
    }));
    let config = RouterConfig {
        cloud_slots: 2,
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
    for c in "umumumu".chars() {
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
