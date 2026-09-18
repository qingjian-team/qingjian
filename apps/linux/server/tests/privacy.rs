//! 隐私能力变化不能把暂存输入泄露到普通日志、用户词频或个人 n-gram。
use qingjian_core::{Engine, Learner};
use qingjian_dictionary::Dictionary;
use qingjian_learning::{FrequencyLearner, InputLog};
use qingjian_linux_server::{Router, RouterConfig};
use qingjian_platform::protocol::{
    ClientMessage, KeyEvent, KeyModifiers, PROTOCOL_VERSION, ServerMessage, SessionId,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
fn setup() -> (Router, PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "qingjian-privacy-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&path).unwrap();
    let dictionary = Dictionary::parse("你好\tni hao\t100\n开发\tkai fa\t100\n").unwrap();
    let engine = Engine::new(dictionary)
        .with_learner(Box::new(
            FrequencyLearner::from_path(path.join("user.tsv")).unwrap(),
        ))
        .with_input_logger(Box::new(InputLog::open(path.join("input-log.jsonl"))));
    (Router::new(engine, RouterConfig::default()), path)
}
fn privacy(router: &mut Router, id: u64, private: bool) {
    router.handle(ClientMessage::Privacy {
        session: SessionId(id),
        private,
    });
}
fn open(router: &mut Router, id: u64, private: bool) {
    router.handle(ClientMessage::OpenSession {
        session: SessionId(id),
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    privacy(router, id, private);
}
fn text(router: &mut Router, id: u64, text: &str) {
    for c in text.chars() {
        router.handle(ClientMessage::Key {
            session: SessionId(id),
            event: KeyEvent::new(c as u32, Some(c), KeyModifiers::default()),
        });
    }
}
fn commit(router: &mut Router, id: u64) -> Option<String> {
    match router
        .handle(ClientMessage::Commit {
            session: SessionId(id),
        })
        .unwrap()
    {
        ServerMessage::Committed { text, .. } => text,
        _ => panic!("commit result"),
    }
}
fn close(router: &mut Router, id: u64) {
    router.handle(ClientMessage::CloseSession {
        session: SessionId(id),
    });
}
#[test]
fn privacy_boundaries_discard_passthrough_composition_and_learning_chain() {
    let (mut router, path) = setup();
    open(&mut router, 1, true);
    text(&mut router, 1, "SECRET");
    privacy(&mut router, 1, false);
    assert_eq!(commit(&mut router, 1), None);
    privacy(&mut router, 1, true);
    text(&mut router, 1, "nihao");
    privacy(&mut router, 1, false);
    assert_eq!(commit(&mut router, 1), None);
    privacy(&mut router, 1, true);
    text(&mut router, 1, "nihao ");
    privacy(&mut router, 1, false);
    text(&mut router, 1, "kaifa ");
    close(&mut router, 1);
    let log = std::fs::read_to_string(path.join("input-log.jsonl")).unwrap();
    assert!(
        !log.contains("SECRET") && !log.contains("nihao") && !log.contains("你好"),
        "{log}"
    );
    assert!(log.contains("开发"));
    let learner = FrequencyLearner::from_path(path.join("user.tsv")).unwrap();
    assert_eq!(learner.weight("你好"), 0);
    assert_eq!(learner.weight("开发"), 1);
    let ngram = learner.user_ngram().unwrap();
    assert_eq!(ngram.pair(Some("你好"), "开发"), 0);
    assert_eq!(ngram.triple(None, "你好", "开发"), 0);
    drop(router);
    std::fs::remove_dir_all(path).unwrap();
}
#[test]
fn suspended_privacy_changes_discard_only_that_context() {
    let (mut router, path) = setup();
    open(&mut router, 1, true);
    open(&mut router, 2, false);
    text(&mut router, 1, "SECRET");
    text(&mut router, 1, "nihao ");
    text(&mut router, 1, "nihao");
    text(&mut router, 2, "kaifa");
    privacy(&mut router, 1, false);
    assert_eq!(commit(&mut router, 2).as_deref(), Some("kaifa"));
    assert_eq!(commit(&mut router, 1), None);
    text(&mut router, 1, "kaifa ");
    text(&mut router, 1, "nihao");
    text(&mut router, 2, "kaifa");
    privacy(&mut router, 1, true);
    assert_eq!(commit(&mut router, 1), None);
    assert_eq!(commit(&mut router, 2).as_deref(), Some("kaifa"));
    close(&mut router, 1);
    close(&mut router, 2);
    let log = std::fs::read_to_string(path.join("input-log.jsonl")).unwrap();
    assert!(!log.contains("SECRET") && !log.contains("你好"));
    let learner = FrequencyLearner::from_path(path.join("user.tsv")).unwrap();
    assert_eq!(learner.user_ngram().unwrap().pair(Some("你好"), "开发"), 0);
    drop(router);
    std::fs::remove_dir_all(path).unwrap();
}
#[test]
fn switching_privacy_contexts_preserves_both_compositions_and_close_logs_normal_only() {
    let (mut router, path) = setup();
    open(&mut router, 1, true);
    open(&mut router, 2, false);
    open(&mut router, 3, false);
    text(&mut router, 1, "nihao");
    text(&mut router, 2, "kaifa");
    assert_eq!(commit(&mut router, 1).as_deref(), Some("nihao"));
    assert_eq!(commit(&mut router, 2).as_deref(), Some("kaifa"));
    text(&mut router, 1, "SECRET");
    text(&mut router, 2, "NORMALBUFFERA");
    text(&mut router, 3, "NORMALBUFFERB");
    close(&mut router, 2);
    close(&mut router, 1);
    drop(router); // 退出时剩余的挂起/活动会话也需刷新。
    let log = std::fs::read_to_string(path.join("input-log.jsonl")).unwrap();
    assert!(
        log.contains("NORMALBUFFERA") && log.contains("NORMALBUFFERB"),
        "{log}"
    );
    assert!(!log.contains("SECRET") && !log.contains("nihao"), "{log}");
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn capability_events_preserve_empty_input_learning_and_suppress_sensitive_persistence() {
    use serde_json::json;
    let (mut router, path) = setup();
    open(&mut router, 1, true);
    let capability = |router: &mut Router, sensitive, password, disabled| {
        router.handle_linux(json!({"LinuxEvent": {"session": 1, "event": {"Capabilities": {"sensitive": sensitive, "password": password, "disabled": disabled}}}}));
    };
    capability(&mut router, false, false, false);
    text(&mut router, 1, "kaifa ");
    capability(&mut router, true, false, false);
    text(&mut router, 1, "SECRET");
    text(&mut router, 1, "nihao ");
    capability(&mut router, true, true, true);
    capability(&mut router, false, false, false);
    assert_eq!(commit(&mut router, 1), None);
    close(&mut router, 1);
    let log = std::fs::read_to_string(path.join("input-log.jsonl")).unwrap();
    assert!(log.contains("开发") && !log.contains("SECRET") && !log.contains("你好"));
    let learner = FrequencyLearner::from_path(path.join("user.tsv")).unwrap();
    assert_eq!(learner.weight("开发"), 1);
    assert_eq!(learner.weight("你好"), 0);
    assert_eq!(learner.user_ngram().unwrap().pair(Some("你好"), "开发"), 0);
    drop(router);
    std::fs::remove_dir_all(path).unwrap();
}
