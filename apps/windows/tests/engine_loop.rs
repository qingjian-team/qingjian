//! Server 进程内闭环的集成测试：不经传输层，直接把协议消息喂给 [`Router`]，验证
//! 「敲拼音 → 出候选 → 选词上屏」在本平台（含 Windows）上跑通。用仓库内的样例词库，无需产品数据。

use std::path::PathBuf;

use qingjian_core::Language;
use qingjian_platform::protocol::{
    ClientMessage, Frame, KeyEvent, KeyOutcome, ServerMessage, SessionId,
};
use qingjian_windows::{Router, assembly};

const SESSION: SessionId = SessionId(1);

/// 用样例词库（`assets/sample/`）装一个 Router，开好一个会话。
fn router() -> Router {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dict = root.join("assets/sample/dict.tsv");
    let glossary = root.join("assets/sample/glossary-en.tsv");
    let engine = assembly::assemble(&dict, Some((Language::English, &glossary)))
        .expect("assemble engine from sample data");
    let mut router = Router::new(engine, 9);
    assert_eq!(
        router.handle(ClientMessage::OpenSession { session: SESSION }),
        None
    );
    router
}

/// 一个字母键（`character` 带小写字母，虚拟键码用其大写 ASCII）。
fn letter(c: char) -> KeyEvent {
    KeyEvent::new(c.to_ascii_uppercase() as u32, Some(c), Default::default())
}

/// 一个数字键 1–9。
fn digit(n: u32) -> KeyEvent {
    let c = char::from_digit(n, 10).unwrap();
    KeyEvent::new(0x30 + n, Some(c), Default::default())
}

/// 拆出一次按键的处理结果。
fn key_result(message: Option<ServerMessage>) -> (KeyOutcome, Option<String>, Frame) {
    match message {
        Some(ServerMessage::KeyResult {
            outcome,
            commit,
            frame,
            ..
        }) => (outcome, commit, frame),
        other => panic!("expected KeyResult, got {other:?}"),
    }
}

/// 敲一串字母，返回最后一次的处理结果。
fn type_letters(router: &mut Router, text: &str) -> (KeyOutcome, Option<String>, Frame) {
    let mut last = None;
    for c in text.chars() {
        last = Some(key_result(router.handle(ClientMessage::Key {
            session: SESSION,
            event: letter(c),
        })));
    }
    last.expect("typed at least one letter")
}

/// preedit 各段拼起来的整行。
fn preedit(frame: &Frame) -> String {
    frame.preedit.iter().map(|s| s.text.as_str()).collect()
}

#[test]
fn typing_pinyin_shows_candidates() {
    let mut router = router();
    let (outcome, commit, frame) = type_letters(&mut router, "nihao");

    assert_eq!(outcome, KeyOutcome::Consumed);
    assert_eq!(commit, None);
    assert_eq!(preedit(&frame), "ni'hao");
    let texts: Vec<&str> = frame
        .candidates
        .items
        .iter()
        .map(|c| c.text.as_str())
        .collect();
    assert!(
        texts.contains(&"你好"),
        "候选里应有「你好」，实际：{texts:?}"
    );
}

#[test]
fn selecting_by_digit_commits_and_clears() {
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    // 找到「你好」在当前页的位置，按对应数字键上屏。
    let position = frame
        .candidates
        .items
        .iter()
        .position(|c| c.text == "你好")
        .expect("「你好」在候选页内");
    let (outcome, commit, after) = key_result(router.handle(ClientMessage::Key {
        session: SESSION,
        event: digit(position as u32 + 1),
    }));

    assert_eq!(outcome, KeyOutcome::Consumed);
    assert_eq!(commit.as_deref(), Some("你好"));
    assert!(
        after.is_empty(),
        "上屏后应收起候选，实际 preedit={:?}",
        preedit(&after)
    );
}

#[test]
fn space_commits_first_candidate() {
    let mut router = router();
    type_letters(&mut router, "ni");
    let space = KeyEvent::new(0x20, Some(' '), Default::default());
    let (outcome, commit, after) = key_result(router.handle(ClientMessage::Key {
        session: SESSION,
        event: space,
    }));

    assert_eq!(outcome, KeyOutcome::Consumed);
    assert_eq!(commit.as_deref(), Some("你"), "「ni」首选应是「你」");
    assert!(after.is_empty());
}

#[test]
fn backspace_shrinks_preedit() {
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    assert_eq!(preedit(&frame), "ni'hao");
    let back = KeyEvent::new(0x08, None, Default::default());
    let (outcome, _, after) = key_result(router.handle(ClientMessage::Key {
        session: SESSION,
        event: back,
    }));

    assert_eq!(outcome, KeyOutcome::Consumed);
    assert_eq!(preedit(&after), "ni'ha");
}

#[test]
fn non_letter_without_composing_passes_through() {
    let mut router = router();
    let space = KeyEvent::new(0x20, Some(' '), Default::default());
    let (outcome, commit, frame) = key_result(router.handle(ClientMessage::Key {
        session: SESSION,
        event: space,
    }));

    assert_eq!(outcome, KeyOutcome::Passthrough);
    assert_eq!(commit, None);
    assert!(frame.is_empty());
}
