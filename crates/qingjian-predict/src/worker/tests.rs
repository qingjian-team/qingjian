//! 用本地 HTTP 服务核验实际调用次数、间隔、取消与缓存；不消耗外部 API 用量。

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use qingjian_core::{Prediction, PredictionKind, PredictionRequest};
use serde_json::Value;

use super::Worker;
use crate::chat_client::ChatClient;
use crate::config::PredictConfig;
use crate::usage::ApiUsageStats;

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
    success: bool,
    usage_path: Option<PathBuf>,
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
            let (status, body) = if success {
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
                    "500 Internal Server Error",
                    serde_json::json!({"error": {
                        "message": "test failure", "type": "invalid_request_error"
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
        ChatClient::new(&config, "test".into(), usage_path),
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
fn sentence_switch_changes_the_actual_request_and_drops_unwanted_sentences() {
    with_worker(0, 0, true, None, |requests, responses, calls| {
        let mut current = request(1, true);
        requests.send(Some(current.clone())).unwrap();
        let (_, off) = calls.recv_timeout(Duration::from_secs(2)).unwrap();
        let off_user: Value =
            serde_json::from_str(off["messages"][1]["content"].as_str().unwrap()).unwrap();
        assert_eq!(off_user["want_sentence"], false);
        let off_system = off["messages"][0]["content"].as_str().unwrap();
        assert!(!off_system.contains("sentence 字段"));
        let reply = responses.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(reply.words[0].text, "开发");
        assert_eq!(reply.sentence, None);

        current.sequence = 2;
        current.want_sentence = true;
        requests.send(Some(current)).unwrap();
        let (_, on) = calls.recv_timeout(Duration::from_secs(2)).unwrap();
        let on_user: Value =
            serde_json::from_str(on["messages"][1]["content"].as_str().unwrap()).unwrap();
        assert_eq!(on_user["want_sentence"], true);
        let on_system = on["messages"][0]["content"].as_str().unwrap();
        assert!(on_system.contains("sentence 字段"));
        assert!(off_system.len() < on_system.len());
        let reply = responses.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(reply.sentence.as_deref(), Some("开发输入法"));
        assert!(calls.recv_timeout(Duration::from_millis(100)).is_err());
    });
}

#[test]
fn continued_typing_and_commit_cancel_pending_calls() {
    with_worker(150, 0, true, None, |requests, _, calls| {
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
        with_worker(30, 300, success, None, |requests, _, calls| {
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
            assert!(second.duration_since(first) >= Duration::from_millis(280));
            let user = body["messages"][1]["content"].as_str().unwrap();
            assert!(user.contains("ruanjian"));
            assert_eq!(body["messages"].as_array().unwrap().len(), 2);
            assert!(calls.recv_timeout(Duration::from_millis(350)).is_err());
        });
    }
}

#[test]
fn manual_request_skips_debounce_and_cached_reply_skips_interval() {
    with_worker(1000, 3000, true, None, |requests, responses, calls| {
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
    with_worker(0, 300, true, None, |requests, responses, calls| {
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
fn usage_counts_actual_network_calls_and_not_cached_or_cancelled_requests() {
    let path = std::env::temp_dir().join(format!("qingjian-worker-{}.json", uuid::Uuid::new_v4()));
    with_worker(
        150,
        0,
        true,
        Some(path.clone()),
        |requests, responses, calls| {
            requests.send(Some(request(1, false))).unwrap();
            requests.send(None).unwrap();
            assert!(calls.recv_timeout(Duration::from_millis(200)).is_err());
            assert_eq!(ApiUsageStats::load(&path).unwrap().requests, 0);
            requests.send(Some(request(2, true))).unwrap();
            calls.recv_timeout(Duration::from_secs(2)).unwrap();
            responses.recv_timeout(Duration::from_secs(2)).unwrap();
            requests.send(Some(request(3, true))).unwrap();
            responses.recv_timeout(Duration::from_secs(2)).unwrap();
            assert!(calls.recv_timeout(Duration::from_millis(100)).is_err());
            let stats = ApiUsageStats::load(&path).unwrap();
            assert_eq!(stats.requests, 1);
            assert_eq!(stats.responses, 1);
            assert_eq!(stats.total_tokens, 732);
            assert_eq!(stats.cached_input_tokens, 384);
        },
    );
}

#[test]
fn failed_http_request_is_counted_once_with_unknown_usage() {
    let path = std::env::temp_dir().join(format!("qingjian-worker-{}.json", uuid::Uuid::new_v4()));
    with_worker(
        0,
        0,
        false,
        Some(path.clone()),
        |requests, responses, calls| {
            requests.send(Some(request(1, true))).unwrap();
            calls.recv_timeout(Duration::from_secs(2)).unwrap();
            assert!(responses.recv_timeout(Duration::from_millis(300)).is_err());
            assert!(calls.try_recv().is_err());
            let stats = ApiUsageStats::load(&path).unwrap();
            assert_eq!(stats.requests, 1);
            assert_eq!(stats.failures, 1);
            assert_eq!(stats.unreported_usage, 1);
            assert_eq!(stats.total_tokens, 0);
        },
    );
}
