//! Linux 选区翻译的请求、评审、取消和隐私边界。
use qingjian_core::{
    Engine, Prediction, PredictionKind, PredictionPolicy, PredictionRequest, Predictor,
};
use qingjian_dictionary::Dictionary;
use qingjian_linux_server::{Router, RouterConfig};
use qingjian_platform::protocol::{
    ClientMessage, KeyEvent, KeyModifiers, PROTOCOL_VERSION, SessionId,
};
use serde_json::{Value, json};
use std::sync::mpsc::{self, Receiver, Sender};

struct ChannelPredictor {
    requests: Sender<PredictionRequest>,

    replies: Receiver<Prediction>,
}

impl Predictor for ChannelPredictor {
    fn policy(&self) -> PredictionPolicy {
        PredictionPolicy::default()
    }
    fn submit(&mut self, request: PredictionRequest) {
        self.requests.send(request).unwrap();
    }
    fn poll(&mut self) -> Option<Prediction> {
        self.replies.try_recv().ok()
    }
}

fn setup() -> (Router, Receiver<PredictionRequest>, Sender<Prediction>) {
    let (sender, requests) = mpsc::channel();
    let (replies, receiver) = mpsc::channel();
    let engine = Engine::new(Dictionary::parse("你\tni\t1\n").unwrap()).with_predictor(Box::new(
        ChannelPredictor {
            requests: sender,
            replies: receiver,
        },
    ));
    let mut router = Router::new(engine, RouterConfig::default());
    router.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    event(
        &mut router,
        json!({"Capabilities": {"sensitive": false, "password": false, "disabled": false}}),
    );
    router.handle_linux(json!({"DisplayReporting": {"session": 1, "identity": {
        "generation": 1, "context": "selection", "revision": 0
    }}}));
    (router, requests, replies)
}

fn event(router: &mut Router, value: Value) -> Value {
    router
        .handle_linux(json!({"LinuxEvent": {"session": 1, "event": value}}))
        .unwrap_or(Value::Null)
}

fn key(router: &mut Router, character: char, modifiers: KeyModifiers) -> Value {
    event(
        router,
        json!({"Key": {"event": KeyEvent::new(character as u32, Some(character), modifiers),
        "release": false, "selection_supported": true}}),
    )
}

fn start(router: &mut Router) -> u64 {
    let shortcut = KeyModifiers {
        alt: true,
        shift: true,
        ..Default::default()
    };
    key(router, 'T', shortcut)["RequestSelection"]["request"]
        .as_u64()
        .unwrap()
}

#[test]
fn selected_text_is_translated_and_replaces_selection_only_after_accept() {
    let (mut router, requests, replies) = setup();
    let request = start(&mut router);
    let frame = event(
        &mut router,
        json!({"Selection": {"request": request, "text": "你好"}}),
    );
    assert_eq!(frame["KeyResult"]["outcome"], "Consumed");
    assert_eq!(frame["KeyResult"]["frame"]["translation_pending"], true);
    assert_eq!(
        frame["KeyResult"]["frame"]["candidates"]["items"][0]["text"],
        "翻译中…"
    );
    let sent = requests.try_recv().unwrap();
    assert_eq!(sent.kind, PredictionKind::Translate);
    assert_eq!(sent.text, "你好");
    replies
        .send(Prediction {
            sequence: sent.sequence,
            words: Vec::new(),
            sentence: Some("hello".into()),
        })
        .unwrap();
    let frame = router
        .handle_linux(json!({"Poll": {"session": 1}}))
        .unwrap();
    assert_eq!(
        frame["Update"]["frame"]["candidates"]["items"][0]["text"],
        "hello"
    );
    assert_eq!(frame["Update"]["frame"]["translation_pending"], false);
    let accepted = key(&mut router, ' ', KeyModifiers::default());
    assert_eq!(accepted["KeyResult"]["outcome"], "Consumed");
    assert_eq!(accepted["KeyResult"]["commit"], "hello");
    assert!(
        accepted["KeyResult"]["frame"]["candidates"]["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn absent_selection_private_field_and_old_reply_never_submit_translation() {
    let (mut router, requests, _) = setup();
    let legacy = event(
        &mut router,
        json!({"Key": {"event": KeyEvent::new('t' as u32, Some('t'),
        KeyModifiers { alt: true, shift: true, ..Default::default() }), "release": false}}),
    );
    assert_eq!(legacy["KeyResult"]["outcome"], "Passthrough");
    let first = start(&mut router);
    let empty = event(
        &mut router,
        json!({"Selection": {"request": first, "text": ""}}),
    );
    assert_eq!(empty["KeyResult"]["outcome"], "Passthrough");
    let too_long = start(&mut router);
    let rejected = event(
        &mut router,
        json!({"Selection": {"request": too_long, "text": "字".repeat(501)}}),
    );
    assert_eq!(rejected["KeyResult"]["outcome"], "Passthrough");
    let second = start(&mut router);
    event(&mut router, json!({"Focus": {"focused": false}}));
    let old = event(
        &mut router,
        json!({"Selection": {"request": second, "text": "秘密"}}),
    );
    assert_eq!(old["KeyResult"]["outcome"], "Passthrough");
    event(
        &mut router,
        json!({"Capabilities": {"sensitive": true, "password": false, "disabled": false}}),
    );
    event(&mut router, json!({"Focus": {"focused": true}}));
    assert_eq!(
        key(
            &mut router,
            't',
            KeyModifiers {
                alt: true,
                shift: true,
                ..Default::default()
            }
        )["KeyResult"]["outcome"],
        "Passthrough"
    );
    assert!(requests.try_recv().is_err());
}

#[test]
fn escape_cancels_translation_and_stale_cloud_reply() {
    let (mut router, requests, replies) = setup();
    let request = start(&mut router);
    event(
        &mut router,
        json!({"Selection": {"request": request, "text": "你好"}}),
    );
    let sent = requests.try_recv().unwrap();
    let escaped = event(
        &mut router,
        json!({"Key": {"event": KeyEvent::new(0x1b, None, KeyModifiers::default()), "release": false}}),
    );
    assert_eq!(escaped["KeyResult"]["outcome"], "Consumed");
    replies
        .send(Prediction {
            sequence: sent.sequence,
            words: Vec::new(),
            sentence: Some("hello".into()),
        })
        .unwrap();
    let frame = router
        .handle_linux(json!({"Poll": {"session": 1}}))
        .unwrap();
    assert!(
        frame["Update"]["frame"]["candidates"]["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
