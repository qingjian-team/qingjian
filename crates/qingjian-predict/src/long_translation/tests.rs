//! 用本地兼容接口验证长文分段、上下文、重试与整篇成功后返回。
use super::*;
#[test]
fn splitting_preserves_unicode_paragraphs_and_every_character() {
    let source = format!(
        "{}。\r\n\r\n{}🙂",
        "我想写一篇长文".repeat(250),
        "日本語".repeat(600)
    );
    let parts = chunks(&source, 1000);
    assert!(parts.len() > 2);
    assert_eq!(parts.concat(), source);
    assert!(parts.iter().all(|p| p.chars().count() <= 1000));
}
#[test]
fn prefers_sentence_end_and_handles_empty_input() {
    assert_eq!(
        chunks("一二三。四五六七八九", 6),
        vec!["一二三。", "四五六七八九"]
    );
    assert!(chunks("", 1000).is_empty());
    assert_eq!(last_chars("文🙂日本語", 3), "日本語");
}

use crate::PredictConfig;
use qingjian_core::PredictionKind;
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::{self, JoinHandle};
use std::time::Instant;

fn read_request(stream: &mut TcpStream) -> Value {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut block = [0u8; 4096];
    loop {
        let count = stream.read(&mut block).unwrap();
        assert!(count > 0);
        bytes.extend_from_slice(&block[..count]);
        if let Some(end) = bytes.windows(4).position(|b| b == b"\r\n\r\n") {
            let header = String::from_utf8_lossy(&bytes[..end]);
            let length: usize = header
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse().unwrap())
                })
                .unwrap();
            if bytes.len() >= end + 4 + length {
                return serde_json::from_slice(&bytes[end + 4..end + 4 + length]).unwrap();
            }
        }
    }
}
fn server(replies: Vec<(&'static str, &'static str)>) -> (String, JoinHandle<Vec<Value>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let worker = thread::spawn(move || {
        let mut requests = Vec::new();
        for (sentence, finish) in replies {
            let deadline = Instant::now() + Duration::from_secs(8);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "本地翻译请求未到达");
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("{error}"),
                }
            };
            requests.push(read_request(&mut stream));
            let body = json!({"id":"local-test","object":"chat.completion","created":0,
                "model":"test","choices":[{"index":0,"message":{"role":"assistant",
                "content":json!({"sentence":sentence}).to_string()},"finish_reason":finish}]})
            .to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        }
        requests
    });
    (url, worker)
}
fn client(url: String) -> ChatClient {
    ChatClient::new(
        &PredictConfig {
            base_url: url,
            model: "test".into(),
            terminology: vec!["项目=Project|プロジェクト".into()],
            ..PredictConfig::default()
        },
        "local-test".into(),
    )
}
fn request(text: String) -> PredictionRequest {
    PredictionRequest {
        sequence: 1,
        kind: PredictionKind::Translate,
        before: String::new(),
        after: String::new(),
        pinyin: String::new(),
        letters: String::new(),
        syllables: 0,
        candidates: Vec::new(),
        guess: String::new(),
        max_items: 0,
        want_sentence: false,
        text,
        target_language: "en".into(),
        translation_source: None,
    }
}
fn run(client: &ChatClient, request: &PredictionRequest) -> Result<Reply, PredictError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(translate(client, request))
}
#[test]
fn paragraphs_keep_spacing_and_pass_previous_context_and_terms() {
    let (url, server) = server(vec![
        ("First paragraph.", "stop"),
        ("Second paragraph.", "stop"),
    ]);
    let source = format!("  {}。\n\n{}  ", "甲".repeat(700), "乙".repeat(700));
    let result = run(&client(url), &request(source))
        .unwrap()
        .sentence
        .unwrap();
    assert_eq!(result, "  First paragraph.\n\nSecond paragraph.  ");
    let requests = server.join().unwrap();
    let first: Value =
        serde_json::from_str(requests[0]["messages"][1]["content"].as_str().unwrap()).unwrap();
    let second: Value =
        serde_json::from_str(requests[1]["messages"][1]["content"].as_str().unwrap()).unwrap();
    assert_eq!(first["previous_source"], "");
    assert_eq!(second["previous_translation"], "First paragraph.");
    assert!(second["previous_source"].as_str().unwrap().ends_with('。'));
    assert_eq!(second["terminology"][0], "项目=Project|プロジェクト");
}
#[test]
fn failed_later_chunk_retries_and_never_returns_a_partial_translation() {
    let (url, server) = server(vec![
        ("First paragraph.", "stop"),
        ("", "stop"),
        ("", "stop"),
        ("", "stop"),
    ]);
    let result = run(
        &client(url),
        &request(format!("{}。{}", "甲".repeat(700), "乙".repeat(700))),
    );
    assert!(result.is_err());
    assert_eq!(server.join().unwrap().len(), 4);
}
#[test]
fn truncated_translation_is_retried_before_returning() {
    let (url, server) = server(vec![("Partial", "length"), ("Complete.", "stop")]);
    let result = run(&client(url), &request("完整译文".into())).unwrap();
    assert_eq!(result.sentence.as_deref(), Some("Complete."));
    assert_eq!(server.join().unwrap().len(), 2);
}
#[test]
fn oversized_selection_is_rejected_before_network_access() {
    let result = run(
        &client("http://127.0.0.1:9".into()),
        &request("文".repeat(20_001)),
    );
    assert!(matches!(result, Err(PredictError::TranslationTooLong)));
}
