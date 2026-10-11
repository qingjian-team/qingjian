//! 使用本地缓存验证单语言输出与离线失败，不读取用户配置或访问外部接口。
use crate::{PredictConfig, chat_client::ChatClient, worker::Worker};
use qingjian_core::{PredictionKind, PredictionRequest};
use std::sync::mpsc;
use std::time::Duration;

fn request(source: &str, language: &str) -> PredictionRequest {
    PredictionRequest {
        sequence: 1,
        kind: PredictionKind::Compose,
        before: String::new(),
        after: String::new(),
        pinyin: "ji'he".into(),
        letters: "jihe".into(),
        syllables: 2,
        candidates: vec![source.into()],
        guess: source.into(),
        max_items: 0,
        want_sentence: false,
        text: String::new(),
        target_language: language.into(),
        translation_source: Some(source.into()),
    }
}
#[test]
fn cached_phrase_returns_only_requested_language_without_waiting_for_network() {
    for (language, expected) in [("en", "Group up."), ("ja", "集合しよう。")] {
        let (send, recv) = mpsc::channel();
        let (out, result) = mpsc::channel();
        let config = PredictConfig {
            provider: "offline".into(),
            selection_provider: "offline".into(),
            ..PredictConfig::default()
        };
        let worker = Worker::new(
            recv,
            out,
            ChatClient::new(&config, "offline".into()),
            Duration::from_millis(5),
            0,
        );
        let thread = std::thread::spawn(|| worker.run().unwrap());
        send.send(request("集合", language)).unwrap();
        let reply = result.recv_timeout(Duration::from_secs(2)).unwrap();
        let translation = reply.translation.unwrap();
        assert_eq!(translation.language.code(), language);
        assert_eq!(translation.text, expected);
        assert!(reply.words.is_empty());
        drop(send);
        thread.join().unwrap();
    }
}
#[test]
fn uncached_offline_request_reports_failure_instead_of_an_empty_hanging_review() {
    let (send, recv) = mpsc::channel();
    let (out, result) = mpsc::channel();
    let config = PredictConfig {
        provider: "offline".into(),
        selection_provider: "offline".into(),
        ..PredictConfig::default()
    };
    let worker = Worker::new(
        recv,
        out,
        ChatClient::new(&config, "offline".into()),
        Duration::from_millis(5),
        0,
    );
    let thread = std::thread::spawn(|| worker.run().unwrap());
    send.send(request("不存在的完整短句", "en")).unwrap();
    let reply = result.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(reply.failed);
    assert!(reply.translation.is_none());
    drop(send);
    thread.join().unwrap();
}
