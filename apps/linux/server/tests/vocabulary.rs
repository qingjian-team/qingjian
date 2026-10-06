//! 真实 VocabularyBook 的私密选词与译词快捷键回归。
use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_learning::VocabularyBook;
use qingjian_linux_server::{Router, RouterConfig};
use qingjian_platform::protocol::{
    ClientMessage, Frame, KeyEvent, KeyModifiers, PROTOCOL_VERSION, ServerMessage, SessionId,
};
use qingjian_translate::Glossary;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn open(router: &mut Router, id: u64, private: bool) {
    router.handle(ClientMessage::OpenSession {
        session: SessionId(id),
        app: None,
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
    modifiers: KeyModifiers,
) -> (Option<String>, Frame) {
    match router
        .handle(ClientMessage::Key {
            session: SessionId(id),
            event: KeyEvent::new(code, character, modifiers),
        })
        .unwrap()
    {
        ServerMessage::KeyResult { commit, frame, .. } => (commit, frame),
        _ => panic!("key result"),
    }
}

fn type_text(router: &mut Router, id: u64, text: &str) {
    for c in text.chars() {
        let _ = key(router, id, c as u32, Some(c), KeyModifiers::default());
    }
}

#[test]
fn private_vocabulary_book_stays_empty_for_selection_and_translation_shortcut() {
    let directory = std::env::temp_dir().join(format!(
        "qingjian-vocabulary-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let vocabulary_path = directory.join("user-vocab.tsv");
    let engine = Engine::new(Dictionary::parse("你好\tni hao\t100\n").unwrap())
        .with_translator(Box::new(
            Glossary::parse(qingjian_core::Language::English, "你好\thello\n").unwrap(),
        ))
        .with_vocabulary_tracker(Box::new(VocabularyBook::open(&vocabulary_path)));
    let mut router = Router::new(engine, RouterConfig::default());
    open(&mut router, 1, true);

    type_text(&mut router, 1, "nihao");
    assert_eq!(
        key(&mut router, 1, 0x20, Some(' '), KeyModifiers::default())
            .0
            .as_deref(),
        Some("你好")
    );
    type_text(&mut router, 1, "nihao");
    let shortcut = key(
        &mut router,
        1,
        b'1' as u32,
        Some('1'),
        KeyModifiers {
            alt: true,
            ..KeyModifiers::default()
        },
    );
    assert_eq!(shortcut.0.as_deref(), Some("hello"));
    router.flush_learning();
    assert!(!vocabulary_path.exists());

    open(&mut router, 2, false);
    type_text(&mut router, 2, "nihao");
    let normal_shortcut = key(
        &mut router,
        2,
        b'1' as u32,
        Some('1'),
        KeyModifiers {
            alt: true,
            ..KeyModifiers::default()
        },
    );
    assert_eq!(normal_shortcut.0.as_deref(), Some("hello"));
    router.flush_learning();
    let saved = std::fs::read_to_string(&vocabulary_path).unwrap();
    assert!(saved.contains("en\thello\t0\t1\t1\t"), "{saved}");
    drop(router);
    std::fs::remove_dir_all(directory).unwrap();
}
