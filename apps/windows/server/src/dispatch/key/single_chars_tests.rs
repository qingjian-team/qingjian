//! 单字模式快捷键（#848）：只在组句时拿走，撞了别的组合键时不用。
use super::tests::{compose, key};
use crate::dispatch::{Router, RouterConfig};
use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_platform::KeyCombo;
use qingjian_platform::protocol::{
    ClientMessage, Frame, KeyModifiers, KeyOutcome, PROTOCOL_VERSION, SessionId,
};

fn ctrl() -> KeyModifiers {
    KeyModifiers {
        ctrl: true,
        ..Default::default()
    }
}

fn nihao_router() -> Router {
    let dictionary = "你好\tni hao\t9000\n你\tni\t9000\n尼\tni\t500\n好\thao\t9000\n";
    let mut router = Router::new(
        Engine::new(Dictionary::parse(dictionary).unwrap()),
        RouterConfig::default(),
    );
    router.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    router
}

fn texts(frame: &Frame) -> Vec<&str> {
    frame
        .candidates
        .items
        .iter()
        .map(|c| c.text.as_str())
        .collect()
}

#[test]
fn ctrl_d_shows_only_single_chars_while_composing() {
    let mut router = nihao_router();
    assert_eq!(
        router.config.single_chars,
        Some(KeyCombo::SINGLE_CHARS_DEFAULT)
    );
    // 没在组句：交还应用（书签、复制行）
    assert_eq!(
        key(&mut router, 0x44, None, ctrl()).0,
        KeyOutcome::Passthrough
    );

    compose(&mut router, "nihao", KeyModifiers::default());
    let (outcome, commit, frame) = key(&mut router, 0x44, None, ctrl());
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(commit.is_none());
    assert_eq!(router.engine.composition().text(), "nihao");
    let singles = texts(&frame);
    assert_eq!(singles.first(), Some(&"你"));
    assert!(
        singles.iter().all(|t| t.chars().count() == 1),
        "{singles:?}"
    );

    let (_, _, frame) = key(&mut router, 0x44, None, ctrl());
    assert!(texts(&frame).contains(&"你好"));
}

#[test]
fn disabled_or_clashing_single_chars_key_goes_to_the_app() {
    let mut router = nihao_router();
    router.config.single_chars = None;
    compose(&mut router, "nihao", KeyModifiers::default());
    assert_eq!(
        key(&mut router, 0x44, None, ctrl()).0,
        KeyOutcome::Passthrough
    );

    let mut config = qingjian_platform::Config::default();
    config.shortcut.single_chars = Some(config.shortcut.predict);
    assert!(RouterConfig::from(&config).single_chars.is_none());
    let (first, _) = config.shortcut.translation_keys();
    config.shortcut.single_chars = Some(KeyCombo {
        modifiers: first,
        key: '1',
    });
    assert!(RouterConfig::from(&config).single_chars.is_none());
}
