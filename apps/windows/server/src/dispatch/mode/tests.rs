//! 通过 DLL 协议验证默认模式、应用隔离、重连与设置变更。

use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_platform::protocol::{ClientMessage, PROTOCOL_VERSION, ServerMessage, SessionId};

use crate::dispatch::{Router, RouterConfig, StatusEvent};

fn router() -> Router {
    Router::new(
        Engine::new(Dictionary::default()),
        RouterConfig {
            default_english: true,
            remember_mode_per_app: true,
            ..Default::default()
        },
    )
}

fn open(router: &mut Router, id: u64, app: Option<&str>) {
    router.handle(ClientMessage::OpenSession {
        session: SessionId(id),
        app: app.map(str::to_owned),
        protocol: PROTOCOL_VERSION,
    });
}

fn sync(router: &mut Router, id: u64) -> bool {
    match router.handle(ClientMessage::SyncMode {
        session: SessionId(id),
    }) {
        Some(ServerMessage::ModeSync {
            english: Some(english),
            ..
        }) => english,
        other => panic!("没有模式回包：{other:?}"),
    }
}

fn change(router: &mut Router, id: u64, english: bool) {
    router.handle(ClientMessage::ModeChanged {
        session: SessionId(id),
        english,
    });
}

#[test]
fn apps_start_in_english_and_remember_independent_choices() {
    let mut router = router();
    open(&mut router, 1, Some("notepad.exe"));
    open(&mut router, 2, Some("Code.exe"));
    assert!(sync(&mut router, 1));
    change(&mut router, 1, false);
    assert!(sync(&mut router, 2));
    assert!(!sync(&mut router, 1));
    assert!(sync(&mut router, 2));
    open(&mut router, 3, Some("chrome.exe"));
    assert!(sync(&mut router, 3));
}

#[test]
fn same_app_shares_mode_case_insensitively_and_survives_reopening() {
    let mut router = router();
    open(&mut router, 1, Some("Code.exe"));
    change(&mut router, 1, false);
    open(&mut router, 2, Some("code.EXE"));
    assert!(!sync(&mut router, 2));
    router.handle(ClientMessage::CloseSession {
        session: SessionId(1),
    });
    router.handle(ClientMessage::CloseSession {
        session: SessionId(2),
    });
    open(&mut router, 3, Some("code.exe"));
    assert!(!sync(&mut router, 3));
    // 同一会话因管道断开重新打开，也不能盖掉应用记忆。
    open(&mut router, 3, Some("code.exe"));
    assert!(!sync(&mut router, 3));
}

#[test]
fn unknown_apps_are_isolated_by_session() {
    let mut router = router();
    open(&mut router, 1, None);
    open(&mut router, 2, None);
    open(&mut router, 3, Some(""));
    change(&mut router, 1, false);
    assert!(sync(&mut router, 2));
    assert!(sync(&mut router, 3));
    assert!(!sync(&mut router, 1));
}

#[test]
fn status_bar_changes_only_the_last_active_app() {
    let mut router = router();
    open(&mut router, 1, Some("notepad.exe"));
    open(&mut router, 2, Some("Code.exe"));
    assert!(sync(&mut router, 1));
    router.handle_status_event(StatusEvent::ToggleMode);
    assert!(sync(&mut router, 2));
    assert!(!sync(&mut router, 1));
}

#[test]
fn global_mode_can_start_in_english_without_app_memory() {
    let mut router = router();
    router.config.remember_mode_per_app = false;
    open(&mut router, 1, Some("notepad.exe"));
    open(&mut router, 2, Some("Code.exe"));
    assert!(sync(&mut router, 1));
    change(&mut router, 1, false);
    assert!(!sync(&mut router, 2));
}

#[test]
fn only_mode_policy_changes_reset_remembered_choices() {
    let mut router = router();
    open(&mut router, 1, Some("notepad.exe"));
    change(&mut router, 1, false);
    let mut next = router.config.clone();
    next.full_width = !next.full_width;
    router.reload_mode(&next);
    router.config = next.clone();
    assert!(!sync(&mut router, 1));
    // 关内置英文时丢弃旧状态；再打开从配置的默认英文起。
    next.english_mode = false;
    router.reload_mode(&next);
    router.config = next.clone();
    change(&mut router, 1, true);
    assert!(!sync(&mut router, 1));
    next.english_mode = true;
    router.reload_mode(&next);
    router.config = next.clone();
    assert!(sync(&mut router, 1));
    next.default_english = false;
    router.reload_mode(&next);
    router.config = next.clone();
    assert!(!sync(&mut router, 1));
    change(&mut router, 1, true);
    next.remember_mode_per_app = false;
    router.reload_mode(&next);
    router.config = next;
    assert!(!sync(&mut router, 1));
}

#[test]
fn legacy_defaults_and_new_settings_reach_the_dll() {
    let legacy = Router::new(Engine::new(Dictionary::default()), RouterConfig::default());
    assert!(!legacy.english);
    assert!(!legacy.input_settings().restore_mode);
    let mut router = router();
    assert!(router.input_settings().restore_mode);
    router.config.remember_mode_per_app = false;
    assert!(router.input_settings().restore_mode);
}
