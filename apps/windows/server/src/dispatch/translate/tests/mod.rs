//! 只有选区翻译接入时也能评审，普通输入不能触发替换。

mod blank;
mod waiting;

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use qingjian_core::{Engine, Prediction, PredictionPolicy, PredictionRequest, Predictor};
use qingjian_dictionary::Dictionary;
use qingjian_platform::protocol::{
    ClientMessage, KeyEvent, KeyModifiers, PROTOCOL_VERSION, ScreenRect, ServerMessage, SessionId,
};

use crate::dispatch::{Router, RouterConfig};

const SESSION: SessionId = SessionId(1);
const SELECTION_RECT: ScreenRect = ScreenRect {
    left: 0,
    top: 0,
    right: 1,
    bottom: 1,
};

struct LocalTranslation {
    results: VecDeque<Prediction>,

    answer: Option<String>,

    submissions: Arc<AtomicUsize>,
}

impl Predictor for LocalTranslation {
    fn policy(&self) -> PredictionPolicy {
        PredictionPolicy::default()
    }
    fn submit(&mut self, request: PredictionRequest) {
        self.submissions.fetch_add(1, Ordering::Relaxed);
        self.results.push_back(Prediction {
            sequence: request.sequence,
            words: Vec::new(),
            sentence: self.answer.clone(),
        });
    }
    fn poll(&mut self) -> Option<Prediction> {
        self.results.pop_front()
    }
}

fn translated_selection(ready: bool) -> Router {
    translated_selection_answer(ready, Some("Hello."))
}

fn translated_selection_answer(ready: bool, answer: Option<&str>) -> Router {
    translated_selection_with_config(ready, answer, RouterConfig::default())
}

fn translation_key(router: &Router) -> KeyEvent {
    let combo = router.config.translate_selection;
    KeyEvent::new(
        combo.key.to_ascii_uppercase() as u32,
        Some(combo.key),
        KeyModifiers::from(combo.modifiers),
    )
}

fn translated_selection_with_config(
    ready: bool,
    answer: Option<&str>,
    config: RouterConfig,
) -> Router {
    let (mut router, _) = translation_router(answer, config);
    let request = request_selection(&mut router);
    assert!(matches!(
        send_selection(&mut router, request, "你好。"),
        ServerMessage::KeyResult { commit: None, .. }
    ));
    if ready {
        router.poll_prediction();
        assert_eq!(
            router
                .translation
                .as_ref()
                .and_then(|translation| translation.result.as_deref()),
            Some("Hello.")
        );
    }
    assert!(router.engine.composition().is_empty());
    router
}

fn translation_router(answer: Option<&str>, config: RouterConfig) -> (Router, Arc<AtomicUsize>) {
    let submissions = Arc::new(AtomicUsize::new(0));
    let mut engine = Engine::new(Dictionary::default());
    engine.set_translation_predictor(Some(Box::new(LocalTranslation {
        results: VecDeque::new(),
        answer: answer.map(str::to_owned),
        submissions: Arc::clone(&submissions),
    })));
    assert!(!engine.prediction_enabled());
    let mut router = Router::new(engine, config);
    router.handle(ClientMessage::OpenSession {
        session: SESSION,
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    (router, submissions)
}

fn request_selection(router: &mut Router) -> u64 {
    let reply = router.handle(ClientMessage::Key {
        session: SESSION,
        event: translation_key(router),
    });
    let Some(ServerMessage::RequestSelection {
        session: SESSION,
        request,
    }) = reply
    else {
        panic!("selection request")
    };
    request
}

fn send_selection(router: &mut Router, request: u64, text: &str) -> ServerMessage {
    router
        .handle(ClientMessage::Selection {
            session: SESSION,
            request,
            text: text.to_owned(),
            rect: SELECTION_RECT,
        })
        .expect("selection reply")
}

#[test]
fn configured_shortcuts_start_translation_and_allow_confirmation() {
    for shortcut in ["ctrl+alt+t", "alt+shift+t", "ctrl+shift+y"] {
        let config = RouterConfig {
            translate_selection: shortcut.parse().unwrap(),
            ..Default::default()
        };
        let mut router = translated_selection_with_config(true, Some("Hello."), config);
        let reply = router.handle(ClientMessage::Key {
            session: SessionId(1),
            event: KeyEvent::new(13, None, KeyModifiers::default()),
        });
        assert!(
            matches!(reply, Some(ServerMessage::KeyResult { commit: Some(text), .. }) if text == "Hello."),
            "translation must use the configured shortcut: {shortcut}"
        );
    }
}

#[test]
fn local_translation_waits_for_confirmation_and_then_commits() {
    let mut router = translated_selection(true);
    let reply = router.handle(ClientMessage::Key {
        session: SessionId(1),
        event: KeyEvent::new(13, None, KeyModifiers::default()),
    });
    assert!(
        matches!(reply, Some(ServerMessage::KeyResult { commit: Some(text), .. }) if text == "Hello.")
    );
    assert!(router.translation.is_none());
}

#[test]
fn escape_keeps_the_original_selection() {
    let mut router = translated_selection(true);
    let reply = router.handle(ClientMessage::Key {
        session: SessionId(1),
        event: KeyEvent::new(27, None, KeyModifiers::default()),
    });
    assert!(matches!(
        reply,
        Some(ServerMessage::KeyResult { commit: None, .. })
    ));
    assert!(router.translation.is_none());
}

#[test]
fn cancelling_while_waiting_discards_the_late_local_translation() {
    let mut router = translated_selection(false);
    router.handle(ClientMessage::Key {
        session: SessionId(1),
        event: KeyEvent::new(27, None, KeyModifiers::default()),
    });
    assert!(router.translation.is_none());
    assert!(router.engine.poll_prediction().is_none());
}
