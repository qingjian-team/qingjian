use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_platform::protocol::{
    ClientMessage, Frame, IndicatorCommand, KeyEvent, KeyModifiers, PROTOCOL_VERSION,
    ServerMessage, SessionId,
};

use super::PanelMouse;
use crate::dispatch::{Router, RouterConfig};

const UP: u32 = 0x26;
const DOWN: u32 = 0x28;
const RIGHT: u32 = 0x27;
const TAB: u32 = 0x09;
const ESCAPE: u32 = 0x1B;

fn router() -> Router {
    let mut router = Router::new(
        Engine::new(Dictionary::parse("你\tni\t100\n尼\tni\t90\n").unwrap()),
        RouterConfig::default(),
    );
    router.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    router
}

fn press(router: &mut Router, code: u32, character: Option<char>) -> (Option<String>, Frame) {
    let modifiers = KeyModifiers::default();
    match router.handle(ClientMessage::Key {
        session: SessionId(1),
        event: KeyEvent::new(code, character, modifiers),
    }) {
        Some(ServerMessage::KeyResult { commit, frame, .. }) => (commit, frame),
        other => panic!("key result: {other:?}"),
    }
}

fn text(router: &mut Router, text: &str) {
    for c in text.chars() {
        press(router, c as u32, Some(c));
    }
}

fn section(router: &Router) -> (String, String) {
    let panel = router.engine.symbol_panel().expect("面板开着");
    (
        panel.group().label().to_owned(),
        panel.section().name.clone(),
    )
}

/// 「你」搜得到东西时面板停在搜索组，先换到符号组。
fn to_symbols(router: &mut Router) {
    if section(router).0 == "搜索" {
        press(router, TAB, None);
    }
    assert_eq!(section(router).0, "符号");
}

#[test]
fn up_on_untouched_first_candidate_opens_and_esc_returns() {
    let mut router = router();
    text(&mut router, "ni");
    let (_, frame) = press(&mut router, UP, None);
    assert!(frame.symbol_panel);
    assert!(router.panel_open());
    assert!(router.panel_frame().is_some());

    // 格子里移动、换组
    to_symbols(&mut router);
    press(&mut router, RIGHT, None);
    assert_eq!(router.panel.as_ref().unwrap().highlight(), 1);
    press(&mut router, DOWN, None);
    let group = section(&router).0;
    press(&mut router, TAB, None);
    assert_ne!(section(&router).0, group);
    assert_eq!(router.panel.as_ref().unwrap().highlight(), 0);

    let (commit, frame) = press(&mut router, ESCAPE, None);
    assert!(commit.is_none());
    assert!(!frame.symbol_panel);
    assert!(!router.panel_open());
    assert!(!frame.candidates.items.is_empty());
}

#[test]
fn up_after_moving_highlight_does_not_open() {
    let mut router = router();
    text(&mut router, "ni");
    press(&mut router, DOWN, None);
    press(&mut router, UP, None);
    let (_, frame) = press(&mut router, UP, None);
    assert!(!frame.symbol_panel);
    assert!(!router.panel_open());
}

#[test]
fn space_commits_item_and_eats_the_search_text() {
    let mut router = router();
    text(&mut router, "ni");
    press(&mut router, UP, None);
    let expected = router
        .panel
        .as_ref()
        .and_then(|g| g.candidate(0))
        .map(|c| c.text.clone())
        .unwrap();
    let (commit, frame) = press(&mut router, ' ' as u32, Some(' '));
    assert_eq!(commit.as_deref(), Some(expected.as_str()));
    assert!(frame.is_empty());
    assert!(!router.panel_open());
}

#[test]
fn brackets_switch_sections_and_letters_close_and_compose() {
    let mut router = router();
    text(&mut router, "ni");
    press(&mut router, UP, None);
    to_symbols(&mut router);
    let first = section(&router);
    press(&mut router, ']' as u32, Some(']'));
    assert_ne!(section(&router), first);
    press(&mut router, '[' as u32, Some('['));
    assert_eq!(section(&router), first);

    let (_, frame) = press(&mut router, 'h' as u32, Some('h'));
    assert!(!frame.symbol_panel);
    assert!(!router.panel_open());
    let preedit: String = frame.preedit.iter().map(|s| s.text.as_str()).collect();
    assert!(preedit.starts_with("ni"), "{preedit}");
}

#[test]
fn menu_opens_without_composition_and_mouse_commits() {
    let mut router = router();
    router.handle(ClientMessage::SyncMode {
        session: SessionId(1),
    });
    router.handle(ClientMessage::Indicator {
        session: SessionId(1),
        command: IndicatorCommand::OpenSymbolPanel,
    });
    assert!(router.panel_open());
    assert!(router.indicator_state().symbol_panel);
    // 没在组句也不是空帧：DLL 据此继续把键交过来
    assert!(!router.self_drawn_frame().is_empty());

    router.handle_panel_mouse(PanelMouse::Cell {
        offset: 2,
        double: false,
    });
    assert_eq!(router.panel.as_ref().unwrap().highlight(), 2);
    router.handle_panel_mouse(PanelMouse::Scroll(1));
    router.handle_panel_mouse(PanelMouse::Scroll(-1));
    let expected = router
        .panel
        .as_ref()
        .and_then(|g| g.candidate(2))
        .map(|c| c.text.clone())
        .unwrap();
    router.handle_panel_mouse(PanelMouse::Cell {
        offset: 2,
        double: true,
    });
    assert!(!router.panel_open());
    assert_eq!(router.take_clicked(SessionId(1)), Some(expected));
    assert!(router.self_drawn_frame().is_empty());
}

#[test]
fn close_button_and_escape_without_composition_end_the_frame() {
    let mut router = router();
    router.handle(ClientMessage::SyncMode {
        session: SessionId(1),
    });
    router.handle(ClientMessage::Indicator {
        session: SessionId(1),
        command: IndicatorCommand::OpenSymbolPanel,
    });
    router.handle_panel_mouse(PanelMouse::Close);
    assert!(!router.panel_open());
    assert!(router.self_drawn_frame().is_empty());

    router.handle(ClientMessage::Indicator {
        session: SessionId(1),
        command: IndicatorCommand::OpenSymbolPanel,
    });
    let (_, frame) = press(&mut router, ESCAPE, None);
    assert!(frame.is_empty());
}
