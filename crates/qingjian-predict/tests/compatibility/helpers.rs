//! 有界的本地模拟接口与公共预测入口。

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use qingjian_core::{Prediction, PredictionKind, PredictionRequest, Predictor};
use qingjian_predict::{CloudPredictor, ConnectionTest, PredictConfig, PredictError};
use serde_json::{Value, json};

pub fn success() -> (u16, Value) {
    (
        200,
        json!({
            "id": "compat-test", "object": "chat.completion", "created": 0,
            "model": "mock", "choices": [{"index": 0, "finish_reason": "stop",
                "message": {"role": "assistant",
                    "content": "{\"ok\":true,\"words\":[],\"sentence\":\"你好\"}"}}]
        }),
    )
}

pub fn rejection(parameter: &str) -> (u16, Value) {
    (
        400,
        json!({"error": {
            "message": format!("Unsupported parameter: '{parameter}' is not supported with this model."),
            "type": "invalid_request_error", "param": parameter, "code": "unsupported_parameter"
        }}),
    )
}

pub fn connection(config: &PredictConfig) -> Result<(), PredictError> {
    ConnectionTest::start(config)
        .and_then(ConnectionTest::wait)
        .map(|_| ())
}

pub fn prediction(predictor: &mut CloudPredictor, sequence: u64) -> Prediction {
    predictor.submit(PredictionRequest {
        sequence,
        manual: true,
        kind: PredictionKind::Compose,
        before: format!("context-{sequence}"),
        after: String::new(),
        pinyin: "ni'hao".to_owned(),
        letters: "nihao".to_owned(),
        syllables: 2,
        candidates: Vec::new(),
        guess: String::new(),
        max_items: 2,
        want_sentence: true,
        text: String::new(),
        target_language: String::new(),
    });
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(reply) = predictor.poll() {
            assert_eq!(reply.sequence, sequence);
            return reply;
        }
        assert!(
            Instant::now() < deadline,
            "public predictor returned no result"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

pub fn with_server<R>(
    replies: Vec<(u16, Value)>,
    test: impl FnOnce(&PredictConfig) -> R,
) -> (R, Vec<Value>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let config = PredictConfig {
        base_url: format!("http://{}/v1", listener.local_addr().unwrap()),
        model: "unlisted-model".to_owned(),
        api_key: Some("test-only-key".to_owned()),
        timeout_ms: 1000,
        debounce_ms: 0,
        min_interval_ms: 0,
        ..PredictConfig::default()
    };
    let stop = Arc::new(AtomicBool::new(false));
    let stopping = stop.clone();
    let server = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut replies = replies.into_iter();
        let mut calls = Vec::new();
        while !stopping.load(Ordering::Relaxed) && Instant::now() < deadline {
            let stream = match listener.accept() {
                Ok((stream, _)) => stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2));
                    continue;
                }
                Err(error) => panic!("accept: {error}"),
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            let mut reader = BufReader::new(stream);
            let mut first_line = String::new();
            reader.read_line(&mut first_line).unwrap();
            assert_eq!(first_line, "POST /v1/chat/completions HTTP/1.1\r\n");
            let mut length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line.trim().is_empty() {
                    break;
                }
                if let Some((name, value)) = line.split_once(':')
                    && name.eq_ignore_ascii_case("content-length")
                {
                    length = value.trim().parse::<usize>().unwrap();
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            calls.push(serde_json::from_slice(&body).unwrap());
            let (status, body) = replies.next().unwrap_or_else(|| {
                (
                    500,
                    json!({"error": {"message": "unexpected extra request"}}),
                )
            });
            let body = body.to_string();
            write!(reader.get_mut(),
                "HTTP/1.1 {status} Test\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()).unwrap();
        }
        calls
    });
    let result = catch_unwind(AssertUnwindSafe(|| test(&config)));
    stop.store(true, Ordering::Relaxed);
    let calls = server.join().unwrap();
    match result {
        Ok(value) => (value, calls),
        Err(panic) => resume_unwind(panic),
    }
}
