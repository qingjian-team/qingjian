//! 雾凇输入码经平台按键分派后仍由原生 Engine 生成、提交候选。

#[path = "../../../../crates/qingjian-core/tests/ice_support/mod.rs"]
mod ice_support;

use ice_support::Fixture;
use qingjian_platform::protocol::{
    ClientMessage, Frame, KeyEvent, KeyModifiers, KeyOutcome, PROTOCOL_VERSION, ServerMessage,
    SessionId,
};
use qingjian_platform::{Config, RimeIceConfig};
use qingjian_windows_server::{Router, RouterConfig};

fn send(
    router: &mut Router,
    code: u32,
    character: Option<char>,
    shift: bool,
) -> (Option<String>, Frame) {
    let event = KeyEvent::new(
        code,
        character,
        KeyModifiers {
            shift,
            ..KeyModifiers::default()
        },
    );
    match router
        .handle(ClientMessage::Key {
            session: SessionId(1),
            event,
        })
        .unwrap()
    {
        ServerMessage::KeyResult {
            outcome,
            commit,
            frame,
            ..
        } => {
            assert_eq!(outcome, KeyOutcome::Consumed);
            (commit, frame)
        }
        other => panic!("unexpected response: {other:?}"),
    }
}

fn input(router: &mut Router, text: &str) -> Frame {
    let mut frame = Frame::default();
    for c in text.chars() {
        let code = match c {
            '+' => 0xbb,
            '(' => 0x39,
            ')' => 0x30,
            '.' => 0xbe,
            '`' => 0xc0,
            _ => c.to_ascii_uppercase() as u32,
        };
        let shift = c.is_ascii_uppercase() || matches!(c, '+' | '(' | ')');
        let (commit, next) = send(router, code, Some(c), shift);
        assert!(commit.is_none(), "premature commit at {c} in {text}");
        frame = next;
    }
    frame
}

#[test]
fn date_tools_symbols_and_shifted_prefixes_survive_key_dispatch() {
    let fixture = Fixture::new();
    let config = Config {
        rime_ice: RimeIceConfig {
            enabled: true,
            data_dir: fixture.path().to_owned(),
        },
        ..Config::default()
    };
    let mut router = Router::new(fixture.engine(), RouterConfig::from(&config));
    router.engine_mut().set_english_mode(true);
    let opened = router.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: Some("native-ice-test".to_owned()),
        protocol: PROTOCOL_VERSION,
    });
    assert!(
        matches!(opened,Some(ServerMessage::SessionOpened {input,..}) if input.shift_letter_compose)
    );
    router.handle(ClientMessage::Privacy {
        session: SessionId(1),
        private: false,
    });
    for (code, expected) in [
        ("nihao", "你好"),
        ("cC1+2", "3"),
        ("cCsin(pi/2)", "1"),
        ("U4e2d", "中"),
        ("N20260217", "丙午马年正月初一"),
        ("uUrenmu", "休"),
        ("xiu`ren", "休"),
        ("va", "ā"),
        ("R1234.5678", "一千二百三十四点五六七八"),
    ] {
        // 每次从空输入开始，候选存在但不提前上屏。
        let frame = input(&mut router, code);
        assert!(
            frame.candidates.items.iter().any(|c| c.text == expected),
            "{code}: {:?}",
            frame.candidates
        );
        let (commit, frame) = send(&mut router, 0x20, Some(' '), false);
        assert_eq!(commit.as_deref(), Some(expected), "{code}");
        assert!(frame.is_empty(), "{code}");
    }
    let frame = input(&mut router, "vhelp");
    assert_eq!(frame.candidates.items[0].text, "符号列表");
    assert_eq!(
        send(&mut router, 0x31, Some('1'), false).0.as_deref(),
        Some("符号列表")
    );
}
