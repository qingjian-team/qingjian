//! 本地 HTTP 回放验证真实请求、修订取消与连接限制。

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use super::{local_endpoint, target_language};
use crate::live::{LiveTranslationConfig, LiveTranslator, Translation};

fn config(listener: &TcpListener) -> LiveTranslationConfig {
    LiveTranslationConfig {
        base_url: format!("http://{}", listener.local_addr().unwrap()),
        debounce_ms: 0,
        ..LiveTranslationConfig::default()
    }
}

fn read_request(stream: &mut TcpStream) -> String {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut buffer = [0; 4096];
    loop {
        let read = stream.read(&mut buffer).unwrap();
        assert!(read > 0);
        bytes.extend_from_slice(&buffer[..read]);
        let text = String::from_utf8_lossy(&bytes);
        if let Some((header, body)) = text.split_once("\r\n\r\n") {
            let length: usize = header
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|value| value.trim().parse().unwrap())
                })
                .unwrap();
            if body.len() >= length {
                return text.into_owned();
            }
        }
    }
}

fn reply(stream: &mut TcpStream, text: &str) {
    let body = serde_json::json!({"response": text, "done": true}).to_string();
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
}

fn wait(translator: &mut LiveTranslator) -> Translation {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(reply) = translator.poll() {
            return reply;
        }
        assert!(Instant::now() < deadline, "未收到本地 HTTP 译文");
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn rejects_remote_credentials_and_non_http() {
    for url in [
        "https://localhost:11434",
        "http://example.com",
        "http://localhost.evil.test",
        "http://user@localhost",
        "http://localhost/path",
        "http://localhost?token=x",
    ] {
        assert!(local_endpoint(url).is_err(), "{url}");
    }
    assert_eq!(
        local_endpoint("http://localhost:11434").unwrap().as_str(),
        "http://127.0.0.1:11434/api/generate"
    );
    assert!(local_endpoint("http://[::1]:11434").is_ok());
}

#[test]
fn sends_plain_ollama_request_and_reads_reply() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut translator = LiveTranslator::new(config(&listener)).unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let request = read_request(&mut stream);
        assert!(request.starts_with("POST /api/generate "));
        let body: serde_json::Value =
            serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(body["stream"], false);
        assert_eq!(body["model"], "maternion/hy-mt2:1.8b");
        assert!(body["prompt"].as_str().unwrap().contains("into English"));
        reply(&mut stream, "I am learning.");
        let (mut next, _) = listener.accept().unwrap();
        let request = read_request(&mut next);
        assert!(request.contains("into Chinese"));
        reply(&mut next, "我正在学习。");
    });
    let id = translator.submit("我正在学习");
    let result = wait(&mut translator);
    assert_eq!(result.id, id);
    assert_eq!(result.result.unwrap(), "I am learning.");
    // 首次结果完成后线程已进入阻塞等待，第二次输入仍可立即唤醒。
    let id = translator.submit("I am learning.");
    let result = wait(&mut translator);
    assert_eq!(result.id, id);
    assert_eq!(result.result.unwrap(), "我正在学习。");
    server.join().unwrap();
}

#[test]
fn cancels_slow_request_and_returns_latest_without_waiting_for_old() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut translator = LiveTranslator::new(config(&listener)).unwrap();
    let (started, received) = mpsc::channel();
    let server = thread::spawn(move || {
        let (mut old, _) = listener.accept().unwrap();
        read_request(&mut old);
        started.send(()).unwrap();
        let (mut new, _) = listener.accept().unwrap();
        let request = read_request(&mut new);
        assert!(request.contains("新句"));
        reply(&mut new, "new");
        // 旧连接直到新译文返回后才结束：若请求未取消，新请求永远不会到达。
        reply(&mut old, "stale");
    });
    translator.submit("旧句");
    received.recv_timeout(Duration::from_secs(2)).unwrap();
    let id = translator.submit("新句");
    let result = wait(&mut translator);
    assert_eq!(result.id, id);
    assert_eq!(result.result.unwrap(), "new");
    translator.cancel();
    assert!(translator.poll().is_none());
    server.join().unwrap();
}

#[test]
fn timeout_is_a_visible_error() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut settings = config(&listener);
    settings.timeout_ms = 100;
    let mut translator = LiveTranslator::new(settings).unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        read_request(&mut stream);
        thread::sleep(Duration::from_millis(300));
    });
    translator.submit("你好");
    assert!(wait(&mut translator).result.unwrap_err().contains("超时"));
    server.join().unwrap();
}

#[test]
fn mixed_sentences_use_the_main_language_instead_of_the_first_character() {
    for source in [
        "API 接口需要鉴权",
        "Ollama 模型已经安装好了",
        "版本2需要3个API",
        "hello你好",
    ] {
        assert_eq!(target_language(source), "English", "{source}");
    }
    for source in [
        "I said 中文 yesterday",
        "English with a 中文 quote",
        "The API requires authentication.",
        "123!?",
    ] {
        assert_eq!(target_language(source), "Chinese", "{source}");
    }
}
