//! 本地 HTTP 回归：实际用量、失败、空正文与兼容重试逐次计数。

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

use serde_json::{Value, json};

use super::scratch;
use crate::chat_client::ChatClient;
use crate::{ApiUsageStats, ConnectionTest, PredictConfig};

fn reply(content: Option<&str>) -> Value {
    json!({
        "id": "test", "object": "chat.completion", "created": 0,
        "model": "test", "usage": {
            "prompt_tokens": 300, "completion_tokens": 12, "total_tokens": 312,
            "prompt_cache_hit_tokens": 128
        }, "choices": [{"index": 0, "finish_reason": "stop",
            "message": {"role": "assistant", "content": content}}]
    })
}

fn with_server(
    replies: Vec<(u16, Value)>,
    test: impl FnOnce(&PredictConfig, &Path, &mpsc::Receiver<Value>),
) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let config = PredictConfig {
        base_url: format!("http://{}", listener.local_addr().unwrap()),
        api_key: Some("test-key".into()),
        timeout_ms: 2000,
        ..PredictConfig::default()
    };
    let (sent, calls) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let stopping = stop.clone();
    let server = std::thread::spawn(move || {
        let mut replies = replies.into_iter();
        while !stopping.load(Ordering::Relaxed) {
            let (mut stream, _) = match listener.accept() {
                Ok(value) => value,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2));
                    continue;
                }
                Err(error) => panic!("accept: {error}"),
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut reader = BufReader::new(&stream);
            let mut length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                if let Some((name, value)) = line.split_once(':')
                    && name.eq_ignore_ascii_case("content-length")
                {
                    length = value.trim().parse().unwrap();
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            sent.send(serde_json::from_slice(&body).unwrap()).unwrap();
            let (status, body) = replies.next().expect("unexpected extra HTTP request");
            let body = body.to_string();
            write!(stream, "HTTP/1.1 {status} Test\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
    });
    let path = scratch();
    test(&config, &path, &calls);
    stop.store(true, Ordering::Relaxed);
    server.join().unwrap();
}

fn chat(config: &PredictConfig, path: &Path) -> Result<String, crate::PredictError> {
    let client = ChatClient::new(config, "test-key".into()).with_usage(Some(path.to_owned()));
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(client.chat("test", "test", 32))
}

#[test]
fn successful_http_and_empty_content_both_record_returned_usage() {
    for content in [Some("{}"), None] {
        with_server(vec![(200, reply(content))], |config, path, calls| {
            assert_eq!(chat(config, path).is_ok(), content.is_some());
            assert_eq!(calls.try_iter().count(), 1);
            let stats = ApiUsageStats::load(path).unwrap();
            assert_eq!(stats.requests, 1);
            assert_eq!(stats.responses, 1);
            assert_eq!(stats.failures, 0);
            assert_eq!(stats.total_tokens, 312);
            assert_eq!(stats.cached_input_tokens, 128);
        });
    }
}

#[test]
fn http_failure_is_one_attempt_and_unknown_usage() {
    with_server(
        vec![(500, json!({"error": {"message": "failure"}}))],
        |config, path, calls| {
            assert!(chat(config, path).is_err());
            assert_eq!(calls.try_iter().count(), 1);
            let stats = ApiUsageStats::load(path).unwrap();
            assert_eq!(stats.requests, 1);
            assert_eq!(stats.failures, 1);
            assert_eq!(stats.unreported_usage, 1);
            assert_eq!(stats.unresolved_requests(), 0);
            assert_eq!(stats.total_tokens, 0);
        },
    );
}

#[test]
fn thinking_fallback_counts_both_attempts_and_successful_usage() {
    with_server(
        vec![
            (
                400,
                json!({"error": {"message": "model always think; cannot disable"}}),
            ),
            (200, reply(Some("{}"))),
        ],
        |config, path, calls| {
            assert!(chat(config, path).is_ok());
            let sent: Vec<_> = calls.try_iter().collect();
            assert_eq!(sent.len(), 2);
            assert!(sent[0].get("reasoning_effort").is_some());
            assert!(sent[1].get("reasoning_effort").is_none());
            let stats = ApiUsageStats::load(path).unwrap();
            assert_eq!(stats.requests, 2);
            assert_eq!(stats.responses, 1);
            assert_eq!(stats.failures, 1);
            assert_eq!(stats.unreported_usage, 1);
            assert_eq!(stats.total_tokens, 312);
        },
    );
}

#[test]
fn connection_test_uses_the_same_usage_recorder() {
    with_server(vec![(200, reply(Some("{}")))], |config, path, calls| {
        let test = ConnectionTest::start_with_usage(config, Some(path.to_owned())).unwrap();
        let start = std::time::Instant::now();
        loop {
            if let Some(result) = test.poll() {
                result.unwrap();
                break;
            }
            assert!(start.elapsed() < Duration::from_secs(3));
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(calls.try_iter().count(), 1);
        assert_eq!(ApiUsageStats::load(path).unwrap().total_tokens, 312);
    });
}
