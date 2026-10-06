//! 用户提供的 librime/Lua 真实协议闭环，不依赖 TSF 注册或候选窗。
use qingjian_core::{Engine, RimeOptions};
use qingjian_dictionary::Dictionary;
use qingjian_platform::protocol::{
    ClientMessage, KeyEvent, KeyOutcome, PROTOCOL_VERSION, ServerMessage, SessionId,
};
use qingjian_windows_server::{Router, RouterConfig};

#[test]
#[ignore = "requires QINGJIAN_TEST_RIME_LIBRARY/SHARED/USER and an isolated profile"]
fn native_scheme_owns_keys_candidates_and_privacy() {
    let mut engine = Engine::new(Dictionary::parse("").unwrap());
    engine
        .enable_rime(RimeOptions {
            library: std::env::var_os("QINGJIAN_TEST_RIME_LIBRARY")
                .unwrap()
                .into(),
            shared_data: std::env::var_os("QINGJIAN_TEST_RIME_SHARED")
                .unwrap()
                .into(),
            user_data: std::env::var_os("QINGJIAN_TEST_RIME_USER").unwrap().into(),
            schema: "rime_ice".into(),
            modules: Vec::new(),
        })
        .unwrap();
    engine.set_english_mode(false);
    let mut router = Router::new(engine, RouterConfig::default());
    let session = SessionId(1);
    let opened = router.handle(ClientMessage::OpenSession {
        session,
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    assert!(matches!(opened, Some(ServerMessage::SessionOpened { input, .. }) if input.rime));
    router.handle(ClientMessage::Privacy {
        session,
        private: false,
    });
    let mut last = None;
    for c in "nihao".chars() {
        last = router.handle(ClientMessage::Key {
            session,
            event: KeyEvent::new(c.to_ascii_uppercase() as u32, Some(c), Default::default()),
        });
    }
    assert!(
        matches!(last, Some(ServerMessage::KeyResult { outcome: KeyOutcome::Consumed, ref frame, .. }) if !frame.preedit.is_empty())
    );
    let committed = router.handle(ClientMessage::Key {
        session,
        event: KeyEvent::new(0x20, Some(' '), Default::default()),
    });
    assert!(
        matches!(committed, Some(ServerMessage::KeyResult { commit: Some(ref text), outcome: KeyOutcome::Consumed, .. }) if text == "你好")
    );
    router.handle(ClientMessage::Privacy {
        session,
        private: true,
    });
    let passed = router.handle(ClientMessage::Key {
        session,
        event: KeyEvent::new(0x4e, Some('n'), Default::default()),
    });
    assert!(matches!(
        passed,
        Some(ServerMessage::KeyResult {
            commit: None,
            outcome: KeyOutcome::Passthrough,
            ..
        })
    ));
}
