//! 简繁切换的组句保留、上屏与快捷键边界。
use super::tests::{compose, key, router};
use crate::dispatch::{Router, RouterConfig};
use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_platform::protocol::{
    ClientMessage, KeyModifiers, KeyOutcome, PROTOCOL_VERSION, SessionId,
};

#[test]
fn traditional_shortcut_preserves_composition_and_changes_commit() {
    let engine = Engine::new(Dictionary::parse("汉语\than yu\t100\n").unwrap());
    let mut router = Router::new(
        engine,
        RouterConfig {
            toggle_traditional: Some(qingjian_platform::KeyCombo::TRADITIONAL_DEFAULT),
            ..Default::default()
        },
    );
    router.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    compose(&mut router, "hanyu", KeyModifiers::default());
    let chord = KeyModifiers {
        ctrl: true,
        shift: true,
        caps: true,
        ..Default::default()
    };
    let (outcome, commit, frame) = key(&mut router, 0x46, Some('F'), chord);
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(commit.is_none());
    assert_eq!(router.engine.composition().text(), "hanyu");
    assert!(
        frame
            .candidates
            .items
            .iter()
            .any(|candidate| candidate.text == "漢語")
    );
    let (_, commit, _) = key(&mut router, 0x20, Some(' '), KeyModifiers::default());
    assert_eq!(commit.as_deref(), Some("漢語"));
    key(&mut router, 0x46, None, chord);
    compose(&mut router, "hanyu", KeyModifiers::default());
    let (_, commit, _) = key(&mut router, 0x20, Some(' '), KeyModifiers::default());
    assert_eq!(commit.as_deref(), Some("汉语"));
}

#[test]
fn disabled_or_rebound_traditional_key_leaves_other_shortcuts_to_app() {
    let mut router = router(9);
    router.config.toggle_traditional = None;
    let chord = KeyModifiers {
        ctrl: true,
        shift: true,
        ..Default::default()
    };
    assert_eq!(
        key(&mut router, 0x46, Some('f'), chord).0,
        KeyOutcome::Passthrough
    );
    router.config.toggle_traditional = Some("ctrl+alt+g".parse().unwrap());
    assert_eq!(
        key(&mut router, 0x46, Some('f'), chord).0,
        KeyOutcome::Passthrough
    );
    let rebound = KeyModifiers {
        ctrl: true,
        alt: true,
        english_mode: true,
        ..Default::default()
    };
    assert_eq!(
        key(&mut router, 0x47, None, rebound).0,
        KeyOutcome::Consumed
    );
    assert!(router.config.traditional);
    let mut config = qingjian_platform::Config::default();
    config.shortcut.toggle_traditional = Some(config.shortcut.translate_selection);
    assert!(RouterConfig::from(&config).toggle_traditional.is_none());
}
