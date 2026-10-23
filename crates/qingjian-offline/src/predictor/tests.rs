//! 失败与普通输入不会触发翻译模型或外部服务。

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use qingjian_core::{PredictionKind, PredictionRequest, Predictor};

use super::NativePredictor;

fn request(kind: PredictionKind, manual: bool) -> PredictionRequest {
    PredictionRequest {
        sequence: 1,
        manual,
        kind,
        before: String::new(),
        after: String::new(),
        pinyin: String::new(),
        letters: String::new(),
        syllables: 0,
        candidates: Vec::new(),
        guess: String::new(),
        max_items: 1,
        want_sentence: true,
        text: "测试".into(),
        target_language: "ja".into(),
    }
}

#[test]
fn ordinary_and_automatic_requests_do_not_start_inference() {
    let mut predictor =
        NativePredictor::new(PathBuf::from("missing"), PathBuf::from("missing")).unwrap();
    predictor.submit(request(PredictionKind::Compose, true));
    predictor.submit(request(PredictionKind::Translate, false));
    assert_eq!(predictor.shared.generation.load(Ordering::Acquire), 0);
    assert!(predictor.poll().is_none());
}

#[test]
fn unsupported_language_reports_failure_without_loading_a_runtime() {
    let mut predictor =
        NativePredictor::new(PathBuf::from("missing"), PathBuf::from("missing")).unwrap();
    predictor.submit(request(PredictionKind::Translate, true));
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(reply) = predictor.poll() {
            assert_eq!(reply.sequence, 1);
            assert!(reply.sentence.is_none());
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
}
