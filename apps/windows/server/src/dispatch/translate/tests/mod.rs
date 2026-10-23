//! 只有选区翻译接入时也能评审，普通输入不能触发替换。

mod fallback;
mod waiting;

use std::collections::VecDeque;

use qingjian_core::{Engine, Prediction, PredictionPolicy, PredictionRequest, Predictor};
use qingjian_dictionary::Dictionary;
use qingjian_platform::protocol::{
    ClientMessage, KeyEvent, KeyModifiers, PROTOCOL_VERSION, ScreenRect, ServerMessage, SessionId,
};

use crate::dispatch::{Router, RouterConfig};

struct LocalTranslation(VecDeque<Prediction>, Option<String>);

impl Predictor for LocalTranslation {
    fn policy(&self) -> PredictionPolicy {
        PredictionPolicy::default()
    }
    fn submit(&mut self, request: PredictionRequest) {
        self.0.push_back(Prediction {
            sequence: request.sequence,
            words: Vec::new(),
            sentence: self.1.clone(),
        });
    }
    fn poll(&mut self) -> Option<Prediction> {
        self.0.pop_front()
    }
}

fn translated_selection(ready: bool) -> Router {
    translated_selection_answer(ready, Some("Hello."))
}

fn translated_selection_answer(ready: bool, answer: Option<&str>) -> Router {
    let mut engine = Engine::new(Dictionary::default());
    engine.set_translation_predictor(Some(Box::new(LocalTranslation(
        VecDeque::new(),
        answer.map(str::to_owned),
    ))));
    assert!(!engine.prediction_enabled());
    let mut router = Router::new(engine, RouterConfig::default());
    router.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    let reply = router.handle(ClientMessage::Key {
        session: SessionId(1),
        event: KeyEvent::new(
            84,
            Some('t'),
            KeyModifiers {
                ctrl: true,
                alt: true,
                ..Default::default()
            },
        ),
    });
    let Some(ServerMessage::RequestSelection { request, .. }) = reply else {
        panic!("selection request")
    };
    let reply = router.handle(ClientMessage::Selection {
        session: SessionId(1),
        request,
        text: "你好。".into(),
        rect: ScreenRect {
            left: 0,
            top: 0,
            right: 1,
            bottom: 1,
        },
    });
    assert!(matches!(
        reply,
        Some(ServerMessage::KeyResult { commit: None, .. })
    ));
    if ready {
        router.poll_prediction();
    }
    if ready {
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
