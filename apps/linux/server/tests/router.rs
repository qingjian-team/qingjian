//! Linux 输入行为与会话隔离回归测试。
use qingjian_core::{CustomPhrase, Engine, Learner};
use qingjian_dictionary::Dictionary;
use qingjian_learning::FrequencyLearner;
use qingjian_linux_server::{Router, RouterConfig};
use qingjian_platform::protocol::{
    ClientMessage, Frame, KeyEvent, KeyModifiers, KeyOutcome, PROTOCOL_VERSION, ServerMessage,
    SessionId,
};

fn router() -> Router {
    let dictionary = Dictionary::parse(
        "你好\tni hao\t100\n开发\tkai fa\t90\n你\tni\t80\n好\thao\t80\n泥\tni\t70\n拟\tni\t60\n",
    )
    .unwrap();
    Router::new(Engine::new(dictionary), RouterConfig::default())
}
fn open(router: &mut Router, id: u64, private: bool) {
    router.handle(ClientMessage::OpenSession {
        session: SessionId(id),
        app: Some("test".into()),
        protocol: PROTOCOL_VERSION,
    });
    router.handle(ClientMessage::Privacy {
        session: SessionId(id),
        private,
    });
}
fn key(
    router: &mut Router,
    id: u64,
    code: u32,
    character: Option<char>,
) -> (KeyOutcome, Option<String>, Frame) {
    match router
        .handle(ClientMessage::Key {
            session: SessionId(id),
            event: KeyEvent::new(code, character, KeyModifiers::default()),
        })
        .unwrap()
    {
        ServerMessage::KeyResult {
            outcome,
            commit,
            frame,
            ..
        } => (outcome, commit, frame),
        _ => panic!("key result"),
    }
}
fn type_text(router: &mut Router, id: u64, text: &str) -> Frame {
    let mut frame = Frame::default();
    for c in text.chars() {
        frame = key(router, id, c as u32, Some(c)).2;
    }
    frame
}
fn preedit(frame: &Frame) -> String {
    frame.preedit.iter().map(|s| s.text.as_str()).collect()
}
#[test]
fn independent_contexts_keep_composition_navigation_and_privacy() {
    let mut router = router();
    open(&mut router, 1, false);
    open(&mut router, 2, true);
    type_text(&mut router, 1, "ni");
    let highlighted = key(&mut router, 1, 0x28, None).2.highlight;
    type_text(&mut router, 2, "kai");
    assert!(router.is_private());
    let frame = key(&mut router, 1, 0, None).2;
    assert_eq!(preedit(&frame), "ni");
    assert_eq!(frame.highlight, highlighted);
    assert!(!router.is_private());
    assert!(
        matches!(router.handle(ClientMessage::Commit { session: SessionId(2) }), Some(ServerMessage::Committed { text: Some(text), .. }) if text == "kai")
    );
    assert_eq!(preedit(&key(&mut router, 1, 0, None).2), "ni");
    router.handle(ClientMessage::CloseSession {
        session: SessionId(2),
    });
    assert_eq!(preedit(&key(&mut router, 1, 0, None).2), "ni");
}
#[test]
fn editing_commit_escape_and_punctuation() {
    let mut router = router();
    open(&mut router, 1, false);
    let frame = type_text(&mut router, 1, "nihao");
    assert_eq!(frame.candidates.items[0].text, "你好");
    assert_eq!(
        key(&mut router, 1, 0x20, Some(' ')).1.as_deref(),
        Some("你好")
    );
    assert_eq!(key(&mut router, 1, 0, Some(',')).1.as_deref(), Some("，"));
    type_text(&mut router, 1, "nihao");
    key(&mut router, 1, 0x24, None);
    assert_eq!(
        preedit(&key(&mut router, 1, 0x2e, None).2).replace('\'', ""),
        "ihao"
    );
    assert!(key(&mut router, 1, 0x1b, None).2.is_empty());
    assert_eq!(key(&mut router, 1, 0x08, None).0, KeyOutcome::Passthrough);
    type_text(&mut router, 1, "kaifa");
    assert_eq!(key(&mut router, 1, 0x0d, None).1.as_deref(), Some("kaifa"));
}
#[test]
fn sessions_share_one_persistent_learner_and_private_input_does_not_write() {
    let directory = std::env::temp_dir().join(format!("qingjian-learning-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("user.tsv");
    let engine = Engine::new(Dictionary::parse("你好\tni hao\t100\n开发\tkai fa\t100\n").unwrap())
        .with_learner(Box::new(FrequencyLearner::from_path(&path).unwrap()));
    let mut router = Router::new(engine, RouterConfig::default());
    open(&mut router, 1, false);
    open(&mut router, 2, false);
    open(&mut router, 3, true);
    type_text(&mut router, 1, "nihao");
    key(&mut router, 1, 0x20, Some(' '));
    type_text(&mut router, 2, "kaifa");
    key(&mut router, 2, 0x20, Some(' '));
    type_text(&mut router, 3, "nihao");
    key(&mut router, 3, 0x20, Some(' '));
    router.flush_learning();
    let saved = FrequencyLearner::from_path(&path).unwrap();
    assert_eq!(saved.weight("你好"), 1);
    assert_eq!(saved.weight("开发"), 1);
    std::fs::remove_dir_all(directory).unwrap();
}
#[test]
fn handshake_mismatch_and_unknown_session_do_not_change_state() {
    let mut router = router();
    open(&mut router, 1, false);
    type_text(&mut router, 1, "ni");
    router.handle(ClientMessage::OpenSession {
        session: SessionId(2),
        app: None,
        protocol: 0,
    });
    assert_eq!(router.session_count(), 1);
    assert!(
        router
            .handle(ClientMessage::Commit {
                session: SessionId(2)
            })
            .is_none()
    );
    assert_eq!(preedit(&key(&mut router, 1, 0, None).2), "ni");
}

#[test]
fn sparse_custom_slots_keep_labels_and_navigation_skips_empty_cells() {
    let mut engine = Engine::new(Dictionary::parse("你\tni\t100\n").unwrap());
    engine
        .set_custom_phrases(vec![
            CustomPhrase {
                code: "qq".into(),
                text: "首位".into(),
                position: 1,
                enabled: true,
            },
            CustomPhrase {
                code: "qq".into(),
                text: "第九位".into(),
                position: 9,
                enabled: true,
            },
        ])
        .unwrap();
    let mut router = Router::new(
        engine,
        RouterConfig {
            page_size: 5,
            ..RouterConfig::default()
        },
    );
    open(&mut router, 1, false);
    let frame = type_text(&mut router, 1, "qq");
    assert_eq!(frame.candidates.items[0].text, "首位");
    assert!(frame.candidates.items[1].text.is_empty());
    assert_eq!(key(&mut router, 1, b'2' as u32, Some('2')).1, None);
    let frame = key(&mut router, 1, 0x28, None).2;
    assert_eq!(frame.page, 1);
    assert_eq!(frame.highlight, 3);
    assert_eq!(frame.candidates.items[3].text, "第九位");
    assert_eq!(
        key(&mut router, 1, b'4' as u32, Some('4')).1.as_deref(),
        Some("第九位")
    );
    type_text(&mut router, 1, "qq");
    assert_eq!(key(&mut router, 1, 0x22, None).2.highlight, 3);
    assert_eq!(
        key(&mut router, 1, b' ' as u32, Some(' ')).1.as_deref(),
        Some("第九位")
    );
}
