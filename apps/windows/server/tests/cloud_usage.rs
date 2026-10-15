//! Windows 装配回归：联想与译词补充共享累计用量，重新装配后记录保留。

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use qingjian_core::{Engine, Language, Translation, Translator};
use qingjian_dictionary::Dictionary;
use qingjian_predict::{ApiUsageStats, PredictConfig};
use qingjian_windows_server::dispatch::attach_cloud;
use serde_json::{Value, json};

struct MissingEnglish(Arc<AtomicUsize>);

impl Translator for MissingEnglish {
    fn language(&self) -> Language {
        Language::English
    }

    fn translate(&self, _text: &str) -> Option<Translation> {
        None
    }

    fn learn(&mut self, _word: &str, _translation: Translation) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

fn wait_for(mut ready: impl FnMut() -> bool) {
    let start = Instant::now();
    while !ready() {
        assert!(start.elapsed() < Duration::from_secs(3));
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn prediction_gloss_fill_and_reload_keep_one_usage_file() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let config = PredictConfig {
        enabled: true,
        base_url: format!("http://{}", listener.local_addr().unwrap()),
        api_key: Some("test-key".into()),
        debounce_ms: 0,
        timeout_ms: 2000,
        ..PredictConfig::default()
    };
    let server = std::thread::spawn(move || {
        let mut users = Vec::new();
        for _ in 0..3 {
            let start = Instant::now();
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(start.elapsed() < Duration::from_secs(3));
                        std::thread::sleep(Duration::from_millis(2));
                    }
                    Err(error) => panic!("accept: {error}"),
                }
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
            let request: Value = serde_json::from_slice(&body).unwrap();
            let user: Value =
                serde_json::from_str(request["messages"][1]["content"].as_str().unwrap()).unwrap();
            let content = if user.get("letters").is_some() {
                json!({"words": [{"text": "开发", "pinyin": "kai fa"}]})
            } else {
                json!({"items": [{"w": "开发", "pos": "v.", "senses": [{"t": "develop"}]}]})
            };
            users.push(user);
            let response = json!({
                "id": "test", "object": "chat.completion", "created": 0, "model": "test",
                "usage": {"prompt_tokens": 10, "completion_tokens": 2, "total_tokens": 12},
                "choices": [{"index": 0, "finish_reason": "stop", "message": {
                    "role": "assistant", "content": content.to_string()
                }}]
            })
            .to_string();
            write!(stream, "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response}", response.len()).unwrap();
        }
        users
    });
    let path = std::env::temp_dir().join(format!(
        "qingjian-windows-usage-{}-{}.json",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    let learned = Arc::new(AtomicUsize::new(0));
    let mut engine = Engine::new(Dictionary::parse("开发\tkai fa\t100\n").unwrap())
        .with_translator(Box::new(MissingEnglish(learned.clone())));
    attach_cloud(&mut engine, &config, Some(path.clone()));
    engine.set_input("kaifa");
    assert!(engine.request_prediction(None, &[]).is_some());
    wait_for(|| engine.poll_prediction().is_some());
    let candidate = engine.query().unwrap().candidates.items[0].clone();
    engine.commit(&candidate);
    wait_for(|| engine.poll_glosses() == 1);
    assert_eq!(learned.load(Ordering::Relaxed), 1);
    assert_eq!(ApiUsageStats::load(&path).unwrap().requests, 2);

    attach_cloud(&mut engine, &config, Some(path.clone()));
    engine.set_input("kaifa");
    assert!(engine.request_prediction(None, &[]).is_some());
    wait_for(|| engine.poll_prediction().is_some());
    let users = server.join().unwrap();
    assert_eq!(users[0]["letters"], "kaifa");
    assert_eq!(users[1]["words"], json!(["开发"]));
    assert_eq!(users[2]["letters"], "kaifa");
    let stats = ApiUsageStats::load(&path).unwrap();
    assert_eq!(stats.requests, 3);
    assert_eq!(stats.responses, 3);
    assert_eq!(stats.failures, 0);
    assert_eq!(stats.total_tokens, 36);
}
