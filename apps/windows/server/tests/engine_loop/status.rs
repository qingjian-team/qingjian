//! 悬浮状态条随模式、双拼方案与开关变化。

use crate::support::*;

#[test]
fn status_bar_mode_click_is_handed_to_dll_via_sync_mode() {
    let config = RouterConfig {
        status_enabled: true,
        ..RouterConfig::default()
    };
    let mut router = router_with(config);
    let recorder = RecordingStatus::default();
    router.set_status_sink(Box::new(recorder.clone()));
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: false,
    });

    // 点「中」：状态条先翻成「英」，DLL 来取时拿到目标模式，取一次就清。
    router.handle_status_event(StatusEvent::ToggleMode);
    assert_eq!(recorder.calls().last(), Some(&Some("英".to_owned())));
    assert_eq!(
        router.handle(ClientMessage::SyncMode { session: SESSION }),
        Some(ServerMessage::ModeSync {
            session: SESSION,
            english: Some(true),
        })
    );
    assert_eq!(
        router.handle(ClientMessage::SyncMode { session: SESSION }),
        Some(ServerMessage::ModeSync {
            session: SESSION,
            english: None,
        })
    );
}

#[test]
fn status_bar_follows_mode_when_enabled() {
    let config = RouterConfig {
        status_enabled: true,
        ..RouterConfig::default()
    };
    let mut router = router_with(config);
    let recorder = RecordingStatus::default();
    router.set_status_sink(Box::new(recorder.clone()));

    // 中文 → 英文：各刷一次；会话关掉（应用退出）不收；切成别的输入法才收起。
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: false,
    });
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: true,
    });
    router.handle(ClientMessage::CloseSession { session: SESSION });
    assert_eq!(
        recorder.calls(),
        vec![Some("中".to_owned()), Some("英".to_owned())]
    );

    router.handle(ClientMessage::ImeSwitched { session: SESSION });
    assert_eq!(recorder.calls().last(), Some(&None));
}

#[test]
fn status_bar_shows_shuangpin_scheme_in_chinese() {
    let config = RouterConfig {
        status_enabled: true,
        shuangpin: Some(ShuangpinScheme::Xiaohe),
        ..RouterConfig::default()
    };
    let mut router = router_with(config);
    let recorder = RecordingStatus::default();
    router.set_status_sink(Box::new(recorder.clone()));

    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: false,
    });

    assert_eq!(recorder.calls(), vec![Some("中 · 小鹤双拼".to_owned())]);
}

#[test]
fn status_bar_stays_hidden_when_disabled() {
    let mut router = router();
    let recorder = RecordingStatus::default();
    router.set_status_sink(Box::new(recorder.clone()));

    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: false,
    });

    assert_eq!(recorder.calls(), vec![None]);
}
