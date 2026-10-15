//! 用本地 HTTP 服务核验实际调用次数、间隔、取消与缓存；不消耗外部 API 用量。

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use qingjian_core::{Prediction, PredictionKind, PredictionRequest};
use serde_json::Value;

use super::Worker;
use crate::chat_client::ChatClient;
use crate::config::PredictConfig;

fn request(sequence: u64, manual: bool) -> PredictionRequest {
    PredictionRequest {
        sequence,
        manual,
        kind: PredictionKind::Compose,
        before: String::new(),
        after: String::new(),
        pinyin: "kai'fa".into(),
        letters: "kaifa".into(),
        syllables: 2,
        candidates: Vec::new(),
        guess: String::new(),
        max_items: 2,
        want_sentence: false,
        text: String::new(),
        target_language: String::new(),
    }
}

fn with_worker(
    debounce_ms: u64,
    interval_ms: u64,
    status_code: u16,
    test: impl FnOnce(
        &mpsc::Sender<Option<PredictionRequest>>,
        &mpsc::Receiver<Prediction>,
        &mpsc::Receiver<(Instant, Value)>,
    ),
) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let stopped = Arc::new(AtomicBool::new(false));
    let stop = stopped.clone();
    let (calls_tx, calls) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let mut count = 0;
        while !stop.load(Ordering::Relaxed) {
            let (mut stream, _) = match listener.accept() {
                Ok(connection) => connection,
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
            calls_tx
                .send((Instant::now(), serde_json::from_slice(&body).unwrap()))
                .unwrap();
            // 留出飞行窗口，让测试在首个请求返回前取消或替换排队的请求。
            std::thread::sleep(Duration::from_millis(40));
            count += 1;
            let status_code = if status_code == 400 && count > 1 {
                200
            } else {
                status_code
            };
            let (status, body) = if status_code == 200 {
                (
                    "200 OK",
                    serde_json::json!({
                        "id": "test", "object": "chat.completion", "created": 0,
                        "model": "test", "usage": {
                            "prompt_tokens": 678, "completion_tokens": 54, "total_tokens": 732,
                            "prompt_cache_hit_tokens": 384
                        }, "choices": [{"index": 0, "finish_reason": "stop",
                            "message": {"role": "assistant", "content":
                                r#"{"words":[{"text":"开发","pinyin":"kai fa"}],"sentence":"开发输入法"}"#}}]
                    }),
                )
            } else {
                (
                    if status_code == 400 {
                        "400 Bad Request"
                    } else if status_code == 401 {
                        "401 Unauthorized"
                    } else if status_code == 402 {
                        "402 Payment Required"
                    } else if status_code == 403 {
                        "403 Forbidden"
                    } else {
                        "500 Internal Server Error"
                    },
                    serde_json::json!({"error": {
                        "message": if status_code == 400 { "model always thinks; always think is required" } else { "test failure" }, "type": "invalid_request_error"
                    }}),
                )
            };
            let body = body.to_string();
            write!(stream, "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
    });
    let config = PredictConfig {
        base_url: url,
        timeout_ms: 2000,
        ..PredictConfig::default()
    };
    let (requests, request_rx) = mpsc::channel();
    let (response_tx, responses) = mpsc::channel();
    let worker = Worker::new(
        request_rx,
        response_tx,
        ChatClient::new(&config, "test".into()),
        Duration::from_millis(debounce_ms),
        Duration::from_millis(interval_ms),
    );
    let worker = std::thread::spawn(move || worker.run().unwrap());
    test(&requests, &responses, &calls);
    drop(requests);
    worker.join().unwrap();
    stopped.store(true, Ordering::Relaxed);
    server.join().unwrap();
}

#[test]
fn continued_typing_and_commit_cancel_pending_calls() {
    with_worker(150, 0, 200, |requests, _, calls| {
        for seq in 1..=10 {
            requests.send(Some(request(seq, false))).unwrap();
            std::thread::sleep(Duration::from_millis(10));
        }
        requests.send(None).unwrap();
        assert!(calls.recv_timeout(Duration::from_millis(200)).is_err());
        requests.send(Some(request(11, false))).unwrap();
        let (_, body) = calls.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(
            body["messages"][1]["content"]
                .as_str()
                .unwrap()
                .contains("kaifa")
        );
        requests.send(Some(request(12, false))).unwrap();
        requests.send(None).unwrap();
        assert!(calls.recv_timeout(Duration::from_millis(200)).is_err());
    });
}

#[test]
fn interval_keeps_only_latest_input_and_also_applies_after_failure() {
    for success in [true, false] {
        with_worker(
            30,
            300,
            if success { 200 } else { 500 },
            |requests, _, calls| {
                requests.send(Some(request(1, true))).unwrap();
                let (first, _) = calls.recv_timeout(Duration::from_secs(2)).unwrap();
                let mut newer = request(2, false);
                newer.letters = "ruanj".into();
                requests.send(Some(newer.clone())).unwrap();
                newer.sequence = 3;
                newer.letters = "ruanjian".into();
                newer.manual = true;
                requests.send(Some(newer)).unwrap();
                assert!(calls.recv_timeout(Duration::from_millis(150)).is_err());
                let (second, body) = calls.recv_timeout(Duration::from_secs(2)).unwrap();
                // worker 从发请求前计间隔，假服务器收到第一条可能晚几十毫秒（全量测试并行跑时更明显）
                assert!(second.duration_since(first) >= Duration::from_millis(200));
                let user = body["messages"][1]["content"].as_str().unwrap();
                assert!(user.contains("ruanjian"));
                assert_eq!(body["messages"].as_array().unwrap().len(), 2);
                assert!(calls.recv_timeout(Duration::from_millis(350)).is_err());
            },
        );
    }
}

#[test]
fn manual_request_skips_debounce_and_cached_reply_skips_interval() {
    with_worker(1000, 3000, 200, |requests, responses, calls| {
        requests.send(Some(request(1, true))).unwrap();
        calls.recv_timeout(Duration::from_millis(500)).unwrap();
        let first = responses.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(!first.words.is_empty());
        requests.send(Some(request(2, true))).unwrap();
        let cached = responses.recv_timeout(Duration::from_millis(500)).unwrap();
        assert_eq!(cached.sequence, 2);
        assert_eq!(cached.words, first.words);
        assert!(calls.recv_timeout(Duration::from_millis(100)).is_err());
    });
}

#[test]
fn cancel_during_interval_prevents_a_second_network_call() {
    with_worker(0, 300, 200, |requests, responses, calls| {
        requests.send(Some(request(1, true))).unwrap();
        calls.recv_timeout(Duration::from_secs(2)).unwrap();
        let mut pending = request(2, true);
        pending.letters = "ruanjian".into();
        requests.send(Some(pending)).unwrap();
        responses.recv_timeout(Duration::from_secs(2)).unwrap();
        requests.send(None).unwrap();
        assert!(calls.recv_timeout(Duration::from_millis(400)).is_err());
    });
}

#[test]
fn account_failure_preserves_upstream_ten_minute_pause() {
    for status in [401, 402, 403] {
        with_worker(0, 0, status, |requests, _, calls| {
            requests.send(Some(request(1, true))).unwrap();
            calls.recv_timeout(Duration::from_secs(2)).unwrap();
            let mut next = request(2, true);
            next.letters = "ruanjian".into();
            requests.send(Some(next)).unwrap();
            assert!(calls.recv_timeout(Duration::from_millis(250)).is_err());
        });
    }
}

#[test]
fn thinking_rejection_keeps_the_explicit_single_fallback() {
    with_worker(0, 0, 400, |requests, responses, calls| {
        requests.send(Some(request(1, true))).unwrap();
        let (_, first) = calls.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(first.get("reasoning_effort").is_some());
        let (_, fallback) = calls.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(fallback.get("reasoning_effort").is_none());
        assert!(
            !responses
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .words
                .is_empty()
        );
        assert!(calls.recv_timeout(Duration::from_millis(100)).is_err());
    });
}
