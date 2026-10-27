//! 面板注释显示词性和分段假名，译词快捷键仍提交原始译文。
use qingjian_core::{Engine, Language};
use qingjian_dictionary::Dictionary;
use qingjian_linux_server::{Router, RouterConfig};
use qingjian_platform::protocol::{
    ClientMessage, KeyEvent, KeyModifiers, PROTOCOL_VERSION, SessionId,
};
use qingjian_translate::Glossary;
use serde_json::{Value, json};

fn router(glosses: &str) -> Router {
    router_with(glosses, RouterConfig::default())
}

fn router_with(glosses: &str, config: RouterConfig) -> Router {
    let engine = Engine::new(Dictionary::parse("开发\tkai fa\t100\n").unwrap()).with_translator(
        Box::new(Glossary::parse(Language::Japanese, glosses).unwrap()),
    );
    let mut router = Router::new(engine, config);
    router.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    router.handle_linux(json!({"DisplayReporting": {
        "session": 1,
        "identity": {"generation": 1, "context": "annotation-test", "revision": 0}
    }}));
    router.handle_linux(
        json!({"LinuxEvent": {"session": 1, "event": {"Capabilities": {
            "sensitive": false, "password": false, "disabled": false
        }}}}),
    );
    router
}

fn key(router: &mut Router, character: char, modifiers: KeyModifiers) -> Value {
    router
        .handle_linux(json!({"LinuxEvent": {"session": 1, "event": {"Key": {
            "event": KeyEvent::new(character as u32, Some(character), modifiers),
            "release": false
        }}}}))
        .unwrap()["KeyResult"]
        .clone()
}

fn compose(router: &mut Router) -> Value {
    let mut result = Value::Null;
    for c in "kaifa".chars() {
        result = key(router, c, KeyModifiers::default());
    }
    assert_eq!(result["frame"]["candidates"]["items"][0]["text"], "开发");
    result
}

#[test]
fn annotations_keep_part_of_speech_and_segment_readings_without_changing_raw_text() {
    for (gloss, text, expected) in [
        (
            "v. 開発する|かいはつする",
            "開発する",
            "v. 開発(かいはつ)する",
        ),
        (
            "食事を始める|しょくじをはじめる",
            "食事を始める",
            "食事(しょくじ)を始(はじ)める",
        ),
        (
            "int. こんにちは|こんにちは",
            "こんにちは",
            "int. こんにちは",
        ),
        ("n. 開発", "開発", "n. 開発"),
        ("hello", "hello", "hello"),
        ("v. 開発する|かいはつ", "開発する", "v. 開発する(かいはつ)"),
    ] {
        let mut router = router(&format!("开发\t{gloss}\n"));
        let result = compose(&mut router);
        let sense = &result["frame"]["candidates"]["items"][0]["translation"]["senses"][0];
        assert_eq!(sense["text"], text);
        assert_eq!(sense["annotation"], expected);
        let polled = router
            .handle_linux(json!({"Poll": {"session": 1}}))
            .unwrap();
        assert_eq!(polled["Update"]["frame"], result["frame"]);
        assert_eq!(polled["Update"]["identity"], result["identity"]);
    }
}

#[test]
fn translation_shortcuts_commit_only_the_selected_raw_sense() {
    // 译词键的缺省按编译平台取（Windows 上是 Ctrl），这里按 Linux 的 Alt 配死，在哪台机器上跑都一样
    let alt = KeyModifiers {
        alt: true,
        ..Default::default()
    };
    let config = RouterConfig {
        translation_keys: (alt, KeyModifiers { shift: true, ..alt }),
        ..RouterConfig::default()
    };
    for (shift, expected) in [(false, "開発する"), (true, "開発")] {
        let mut router = router_with(
            "开发\tv. 開発する|かいはつする\tn. 開発|かいはつ\n",
            config.clone(),
        );
        let result = compose(&mut router);
        assert_eq!(
            result["frame"]["candidates"]["items"][0]["translation"]["senses"][0]["annotation"],
            "v. 開発(かいはつ)する"
        );
        let commit = key(
            &mut router,
            '1',
            KeyModifiers {
                alt: true,
                shift,
                ..Default::default()
            },
        );
        assert_eq!(commit["commit"], expected);
        assert!(
            commit["frame"]["candidates"]["items"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn normal_selection_and_missing_translations_keep_their_existing_behavior() {
    let mut translated = router("开发\tv. 開発する|かいはつする\n");
    compose(&mut translated);
    assert_eq!(
        key(&mut translated, '1', KeyModifiers::default())["commit"],
        "开发"
    );

    let mut untranslated = router("");
    let result = compose(&mut untranslated);
    assert!(result["frame"]["candidates"]["items"][0]["translation"].is_null());
    let result = key(
        &mut untranslated,
        '1',
        KeyModifiers {
            alt: true,
            ..Default::default()
        },
    );
    assert!(result["commit"].is_null());
}

#[test]
fn turning_off_part_of_speech_keeps_only_the_translation_and_readings() {
    let config = RouterConfig {
        translation_pos: false,
        ..RouterConfig::default()
    };
    let mut router = router_with("开发\tv. 開発する|かいはつする\n", config);
    let result = compose(&mut router);
    let sense = &result["frame"]["candidates"]["items"][0]["translation"]["senses"][0];
    assert_eq!(sense["text"], "開発する");
    assert_eq!(sense["annotation"], "開発(かいはつ)する");
}
