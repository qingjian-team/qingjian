//! 符号面板翻译组：↑ 带着拼音进框、选词进框、停一下译文进应用的 preedit、回车上屏；没在打字时快捷键打开。

use std::time::Duration;

use qingjian_core::{Prediction, PredictionKind, PredictionPolicy, PredictionRequest, Predictor};
use qingjian_platform::KeyCombo;

use crate::support::*;

/// 翻译请求一来就回一条固定译文的假提供方。
struct EchoTranslator {
    replies: Vec<Prediction>,

    submitted: Arc<Mutex<Vec<PredictionRequest>>>,
}

impl Predictor for EchoTranslator {
    fn policy(&self) -> PredictionPolicy {
        PredictionPolicy {
            slots: 0,
            max_items: 0,
            sentence: false,
            ..PredictionPolicy::default()
        }
    }

    fn submit(&mut self, request: PredictionRequest) {
        self.submitted.lock().unwrap().push(request.clone());
        if request.kind == PredictionKind::Translate {
            self.replies.push(Prediction {
                sequence: request.sequence,
                words: Vec::new(),
                sentence: Some("Hello".to_owned()),
            });
        }
    }

    fn poll(&mut self) -> Option<Prediction> {
        self.replies.pop()
    }
}

fn translating_router() -> (Router, Arc<Mutex<Vec<PredictionRequest>>>) {
    let submitted = Arc::new(Mutex::new(Vec::new()));
    let mut router = router_with(RouterConfig {
        cloud_enabled: true,
        ..RouterConfig::default()
    });
    router.engine_mut().set_predictor(Box::new(EchoTranslator {
        replies: Vec::new(),
        submitted: submitted.clone(),
    }));
    (router, submitted)
}

fn poll(router: &mut Router) -> Frame {
    match router.handle(ClientMessage::Poll { session: SESSION }) {
        Some(ServerMessage::Update { frame, .. }) => frame,
        other => panic!("expected Update, got {other:?}"),
    }
}

fn translations(submitted: &Arc<Mutex<Vec<PredictionRequest>>>) -> Vec<String> {
    submitted
        .lock()
        .unwrap()
        .iter()
        .filter(|r| r.kind == PredictionKind::Translate)
        .map(|r| r.text.clone())
        .collect()
}

#[test]
fn up_with_pinyin_types_into_the_box_and_enter_commits_the_translation() {
    let (mut router, submitted) = translating_router();
    type_letters(&mut router, "nihao");
    let (_, _, frame) = press(&mut router, function_key(0x26));
    assert!(frame.symbol_panel);
    // 拼音画在框里，应用里还没有译文
    assert_eq!(preedit(&frame), "");
    let (_, commit, frame) = press(&mut router, letter(' '));
    assert_eq!(commit, None);
    assert!(frame.symbol_panel);
    assert!(translations(&submitted).is_empty());

    std::thread::sleep(Duration::from_millis(900));
    poll(&mut router);
    let frame = poll(&mut router);
    assert_eq!(translations(&submitted), ["你好"]);
    assert_eq!(preedit(&frame), "Hello");

    let (_, commit, frame) = press(&mut router, function_key(0x0D));
    assert_eq!(commit.as_deref(), Some("Hello"));
    assert!(frame.is_empty());
}

#[test]
fn shortcut_opens_the_box_when_not_typing() {
    let (mut router, _) = translating_router();
    let combo = KeyCombo::TRANSLATE_PANEL_DEFAULT;
    let event = KeyEvent::new(
        combo.key.to_ascii_uppercase() as u32,
        Some(combo.key),
        combo.modifiers.into(),
    );
    let (outcome, _, frame) = press(&mut router, event);
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(frame.symbol_panel);
    // Esc 空框收起
    let (_, _, frame) = press(&mut router, function_key(0x1B));
    assert!(frame.is_empty());
}

#[test]
fn shift_arrows_and_ctrl_a_select_in_the_box() {
    let (mut router, _) = translating_router();
    type_letters(&mut router, "nihao");
    let (_, _, frame) = press(&mut router, function_key(0x26));
    // 还在打拼音：Ctrl+A 照常归应用
    assert!(!frame.panel_text_box);
    let (_, _, frame) = press(&mut router, letter(' '));
    assert!(frame.panel_text_box);
    let shift = KeyModifiers {
        shift: true,
        ..KeyModifiers::default()
    };
    let selection = |router: &mut Router| {
        router
            .engine_mut()
            .symbol_panel()
            .and_then(|panel| panel.translate_selection())
    };
    // Shift+← 选一个字，Shift+Home 拉到开头
    press(&mut router, KeyEvent::new(0x25, None, shift));
    assert_eq!(selection(&mut router), Some(1..2));
    press(&mut router, KeyEvent::new(0x24, None, shift));
    assert_eq!(selection(&mut router), Some(0..2));
    // 不按 Shift 的 → 收起选区
    press(&mut router, function_key(0x27));
    assert_eq!(selection(&mut router), None);
    let ctrl = KeyModifiers {
        ctrl: true,
        ..KeyModifiers::default()
    };
    let (outcome, _, frame) = press(&mut router, KeyEvent::new(0x41, Some('a'), ctrl));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(frame.symbol_panel);
    assert_eq!(selection(&mut router), Some(0..2));
    press(&mut router, function_key(0x08));
    let source = router
        .engine_mut()
        .symbol_panel()
        .map(|panel| panel.translate_source().to_owned());
    assert_eq!(source.as_deref(), Some(""));
}
