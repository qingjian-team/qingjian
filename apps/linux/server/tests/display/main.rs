//! 真正上屏触发曝光；Linux 回报按连接代次/上下文/帧验证，不能越过隐私边界。
mod events;
use qingjian_core::{Engine, Language, Sense, Translation, Translator, VocabularyTracker};
use qingjian_dictionary::Dictionary;
use qingjian_linux_server::{Router, RouterConfig};
use qingjian_platform::protocol::{
    ClientMessage, KeyEvent, KeyModifiers, PROTOCOL_VERSION, SessionId,
};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

struct Glosses;
impl Translator for Glosses {
    fn language(&self) -> Language {
        Language::English
    }
    fn translate(&self, text: &str) -> Option<Translation> {
        (text == "你好").then(|| {
            Translation::new(
                Language::English,
                ["hello", "greetings"]
                    .into_iter()
                    .map(|text| Sense {
                        part_of_speech: None,
                        text: text.into(),
                        reading: None,
                        fresh: false,
                    })
                    .collect(),
            )
        })
    }
}
struct Book(Arc<Mutex<Vec<String>>>);
impl VocabularyTracker for Book {
    fn exposures(&self, _: Language, _: &str) -> u32 {
        0
    }
    fn record_exposure(&mut self, _: Language, word: &str) {
        self.0.lock().unwrap().push(word.into());
    }
    fn record_commit(&mut self, _: Language, _: &str, _: bool) {}
}
fn router() -> (Router, Arc<Mutex<Vec<String>>>) {
    let book = Arc::new(Mutex::new(Vec::new()));
    let engine = Engine::new(Dictionary::parse("你好\tni hao\t100\n").unwrap())
        .with_translator(Box::new(Glosses))
        .with_vocabulary_tracker(Box::new(Book(book.clone())));
    let mut router = Router::new(engine, RouterConfig::default());
    router.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    router.handle(ClientMessage::Privacy {
        session: SessionId(1),
        private: false,
    });
    router.handle_linux(json!({"DisplayReporting": {"session": 1, "identity": {"generation": 2, "context": "first", "revision": 0}}}));
    (router, book)
}
fn key(router: &mut Router, c: char) -> Value {
    router
        .handle_linux(
            serde_json::to_value(ClientMessage::Key {
                session: SessionId(1),
                event: KeyEvent::new(c as u32, Some(c), KeyModifiers::default()),
            })
            .unwrap(),
        )
        .unwrap()
}
fn compose(router: &mut Router) -> Value {
    let mut last = Value::Null;
    for c in "nihao".chars() {
        last = key(router, c);
    }
    assert_eq!(
        last["KeyResult"]["frame"]["candidates"]["items"][0]["text"],
        "你好"
    );
    last["KeyResult"]["identity"].clone()
}
#[test]
fn display_ack_replaces_page_and_deduplicates_senses() {
    let (mut router, book) = router();
    let identity = compose(&mut router);
    router.handle_linux(json!({"DisplayAcknowledged": {"session": 1, "identity": identity, "senses": [[0, 1], [0, 1]]}}));
    key(&mut router, ' ');
    assert_eq!(*book.lock().unwrap(), ["greetings"]);
}
#[test]
fn wrong_identity_or_indices_cannot_create_exposures() {
    for defect in ["generation", "context", "revision", "sense", "row"] {
        let (mut router, book) = router();
        let mut identity = compose(&mut router);
        let mut senses = json!([[0, 0]]);
        match defect {
            "context" => identity[defect] = json!("other"),
            "sense" => senses = json!([[0, 0], [0, 99]]),
            "row" => senses = json!([[99, 0]]),
            _ => identity[defect] = json!(999),
        }
        router.handle_linux(
            json!({"DisplayAcknowledged": {"session": 1, "identity": identity, "senses": senses}}),
        );
        key(&mut router, ' ');
        assert!(book.lock().unwrap().is_empty(), "{defect}");
    }
}
#[test]
fn private_and_old_frame_reports_cannot_be_replayed() {
    let (mut router, book) = router();
    let old = compose(&mut router);
    router.handle(ClientMessage::Privacy {
        session: SessionId(1),
        private: true,
    });
    let private = compose(&mut router);
    for identity in [old, private] {
        router.handle_linux(json!({"DisplayAcknowledged": {"session": 1, "identity": identity, "senses": [[0, 0]]}}));
    }
    key(&mut router, ' ');
    router.handle(ClientMessage::Privacy {
        session: SessionId(1),
        private: false,
    });
    compose(&mut router);
    key(&mut router, ' ');
    assert!(book.lock().unwrap().is_empty());
}

#[test]
fn same_frame_fallback_replaces_exposure_without_double_counting() {
    let (mut router, book) = router();
    let identity = compose(&mut router);
    for senses in [
        json!([[0, 0], [0, 1]]),
        json!([]),
        json!([[0, 0]]),
        json!([[0, 0]]),
    ] {
        router.handle_linux(
            json!({"DisplayAcknowledged": {"session": 1, "identity": identity, "senses": senses}}),
        );
    }
    key(&mut router, ' ');
    assert_eq!(*book.lock().unwrap(), ["hello"]);
}

#[test]
fn poll_invalidates_the_previous_display_until_acknowledged() {
    let (mut router, book) = router();
    let identity = compose(&mut router);
    router.handle_linux(
        json!({"DisplayAcknowledged": {"session": 1, "identity": identity, "senses": [[0, 0]]}}),
    );
    let update = router
        .handle_linux(json!({"Poll": {"session": 1}}))
        .unwrap();
    assert_ne!(identity, update["Update"]["identity"]);
    router.handle_linux(
        json!({"DisplayAcknowledged": {"session": 1, "identity": identity, "senses": [[0, 0]]}}),
    );
    key(&mut router, ' ');
    assert!(book.lock().unwrap().is_empty());
}

#[test]
fn unfocused_or_resumed_session_requires_a_new_display_report() {
    let (mut router, book) = router();
    let identity = compose(&mut router);
    router.handle_linux(
        json!({"DisplayAcknowledged": {"session": 1, "identity": identity, "senses": [[0, 0]]}}),
    );
    router.handle_linux(
        json!({"LinuxEvent": {"session": 1, "event": {"Focus": {"focused": false}}}}),
    );
    router.handle_linux(
        json!({"DisplayAcknowledged": {"session": 1, "identity": identity, "senses": [[0, 0]]}}),
    );
    key(&mut router, ' ');
    assert!(book.lock().unwrap().is_empty());

    router
        .handle_linux(json!({"LinuxEvent": {"session": 1, "event": {"Focus": {"focused": true}}}}));
    let identity = compose(&mut router);
    router.handle_linux(
        json!({"DisplayAcknowledged": {"session": 1, "identity": identity, "senses": [[0, 0]]}}),
    );
    router.handle(ClientMessage::OpenSession {
        session: SessionId(2),
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    router.handle(ClientMessage::Poll {
        session: SessionId(2),
    });
    // 会话恢复不能把失焦前的显示记录带回来。
    key(&mut router, ' ');
    assert!(book.lock().unwrap().is_empty());
}
