//! 真模型与真命名管道联调；不注册 DLL、不切换日用 Server，不能替代应用内 TSF 验收。
#![cfg(windows)]

use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use qingjian_core::Language;
use qingjian_platform::protocol::{KeyEvent, KeyModifiers, ScreenRect, SessionId};
use qingjian_tsf::client::{EngineClient, KeyReply, KeyResponse, pipe::PipeStream};
use qingjian_windows_server::{
    AssemblySpec, LanguageModelFiles, Router, RouterConfig, assembly, dispatch, ipc,
};

fn key(code: u32, character: Option<char>) -> KeyEvent {
    KeyEvent::new(code, character, KeyModifiers::default())
}

fn result(reply: KeyReply) -> KeyResponse {
    match reply {
        KeyReply::Result(result) => result,
        KeyReply::NeedSelection { .. } => panic!("unexpected selection request"),
    }
}

fn select(client: &mut EngineClient<PipeStream>, text: &str) {
    let reply = client
        .key(KeyEvent::new(
            84,
            Some('t'),
            KeyModifiers {
                ctrl: true,
                alt: true,
                ..Default::default()
            },
        ))
        .unwrap();
    let KeyReply::NeedSelection { request } = reply else {
        panic!("native translation must work with cloud disabled")
    };
    let reply = client
        .selection(
            request,
            text.to_owned(),
            ScreenRect {
                left: 0,
                top: 0,
                right: 1,
                bottom: 1,
            },
        )
        .unwrap();
    assert!(
        reply.commit.is_none(),
        "selection must not be replaced before confirmation"
    );
}

fn wait_translation(client: &mut EngineClient<PipeStream>) -> String {
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(30) {
        let (frame, commit) = client.poll().unwrap();
        assert!(
            commit.is_none(),
            "background inference must not commit text"
        );
        if let Some(candidate) = frame.candidates.items.first()
            && candidate.text != "翻译中…"
        {
            return candidate.text.clone();
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("native translation timed out")
}

#[test]
#[ignore = "requires public product data and downloaded ONNX models"]
fn native_model_round_trips_through_the_real_tsf_client_and_pipe() {
    let root =
        PathBuf::from(std::env::var_os("QINGJIAN_NATIVE_TEST_BUNDLE").expect("bundle directory"));
    let user =
        PathBuf::from(std::env::var_os("QINGJIAN_NATIVE_TEST_USER").expect("test model directory"));
    let name = format!(r"\\.\pipe\qingjian-native-test-{}", std::process::id());
    let server_name = name.clone();
    thread::spawn(move || {
        let mut engine = assembly::assemble(&AssemblySpec {
            glossary: Some((
                Language::English,
                root.join("data/generated/glossary-en.qj"),
            )),
            language_model: Some(LanguageModelFiles::Packed(
                root.join("data/generated/lm.qj"),
            )),
            english: Some(root.join("data/generated/english.tsv")),
            ..AssemblySpec::new(root.join("data/generated/dict.qj"))
        })
        .unwrap();
        dispatch::attach_offline_translation(&mut engine, true, Some(&user), &root);
        assert!(!engine.prediction_enabled());
        assert!(engine.translation_enabled());
        let mut router = Router::new(engine, RouterConfig::default());
        let (tx, rx) = std::sync::mpsc::channel();
        ipc::pipe::serve_pipe(&server_name, &mut router, tx, rx).unwrap();
    });
    let stream = (0..100)
        .find_map(|_| {
            let stream = qingjian_tsf::client::pipe::connect(&name).ok();
            if stream.is_none() {
                thread::sleep(Duration::from_millis(50));
            }
            stream
        })
        .expect("test pipe listener");
    let (mut client, _) =
        EngineClient::open(stream, SessionId(1), Some("native-test.exe".into())).unwrap();

    for letter in "nihao".chars() {
        let reply = result(
            client
                .key(key(letter.to_ascii_uppercase() as u32, Some(letter)))
                .unwrap(),
        );
        assert!(reply.commit.is_none());
    }
    thread::sleep(Duration::from_millis(1500));
    let (frame, commit) = client.poll().unwrap();
    assert!(commit.is_none());
    assert!(!frame.preedit.is_empty());
    assert!(
        frame
            .candidates
            .items
            .iter()
            .any(|candidate| candidate.text == "你好")
    );
    assert_eq!(
        result(client.key(key(32, Some(' '))).unwrap())
            .commit
            .as_deref(),
        Some("你好")
    );

    select(&mut client, "你好，今天下午我们一起去公园散步。");
    let waiting = result(client.key(key(13, None)).unwrap());
    assert!(waiting.commit.is_none());
    assert!(
        waiting.reviewing_selection,
        "confirming before inference completes must keep the DLL in translation review"
    );
    let english = wait_translation(&mut client);
    assert!(english.to_lowercase().contains("park"), "{english}");
    assert_eq!(
        result(client.key(key(13, None)).unwrap()).commit.as_deref(),
        Some(english.as_str())
    );
    println!("Chinese -> English confirmed: {english}");

    select(&mut client, "Please confirm the meeting time for tomorrow.");
    let chinese = wait_translation(&mut client);
    assert!(chinese.contains("明天"), "{chinese}");
    assert!(result(client.key(key(27, None)).unwrap()).commit.is_none());
    println!("English -> Chinese cancelled without commit: {chinese}");

    select(&mut client, "请在明天下午三点以前把修改后的文件发送给我。");
    assert!(result(client.key(key(27, None)).unwrap()).commit.is_none());
    for letter in "nihao".chars() {
        assert!(
            result(
                client
                    .key(key(letter.to_ascii_uppercase() as u32, Some(letter)))
                    .unwrap()
            )
            .commit
            .is_none()
        );
    }
    thread::sleep(Duration::from_secs(3));
    let (frame, commit) = client.poll().unwrap();
    assert!(commit.is_none());
    assert!(
        !frame.preedit.is_empty(),
        "cancelled translation must not end normal composition"
    );
    assert_eq!(
        result(client.key(key(32, Some(' '))).unwrap())
            .commit
            .as_deref(),
        Some("你好")
    );
    println!("Normal composition and late-result cancellation passed");
}
