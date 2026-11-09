//! 云联想接进会话：结果插在本地首选之后（先整句、再云端词）、带云朵标记，选整句走 `accept_prediction`。
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use qingjian_android::Session;
use qingjian_core::{
    CloudWord, Engine, Prediction, PredictionPolicy, PredictionRequest, Predictor,
};
use qingjian_dictionary::Dictionary;

fn sample(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../assets/sample")
        .join(name)
}

/// 记下最后一次请求的序号（0 是没有），取结果时回一条整句加一个词。
struct FakeCloud {
    last: Arc<AtomicU64>,
}

impl Predictor for FakeCloud {
    fn policy(&self) -> PredictionPolicy {
        PredictionPolicy {
            automatic: true,
            slots: 2,
            sentence: true,
            ..PredictionPolicy::default()
        }
    }

    fn submit(&mut self, request: PredictionRequest) {
        self.last.store(request.sequence, Ordering::SeqCst);
    }

    fn poll(&mut self) -> Option<Prediction> {
        let sequence = self.last.swap(0, Ordering::SeqCst);
        if sequence == 0 {
            return None;
        }
        Some(Prediction {
            sequence,
            words: vec![CloudWord {
                text: "拟好".to_owned(),
                syllables: vec!["ni".to_owned(), "hao".to_owned()],
                reading: None,
            }],
            sentence: Some("你好世界".to_owned()),
        })
    }
}

fn session(last: Arc<AtomicU64>) -> Session {
    let mut engine = Engine::new(Dictionary::from_path(sample("dict.tsv")).unwrap());
    engine.set_predictor(Box::new(FakeCloud { last }));
    let user_dir = std::env::temp_dir().join(format!("qingjian-prediction-{}", std::process::id()));
    Session::new(engine, sample("wubi86.tsv"), sample(""), user_dir)
}

#[test]
fn cloud_results_follow_the_first_local_candidate() {
    let last = Arc::new(AtomicU64::new(0));
    let mut session = session(Arc::clone(&last));
    session.set_surrounding("今天".to_owned(), String::new());
    for c in "niha".chars() {
        session.push(c);
    }
    let frame = session.push('o');
    assert!(last.load(Ordering::SeqCst) != 0, "组句时发了联想");
    let first = frame.candidates[0].text.clone();

    let frame = session.poll_prediction().expect("结果插进了候选");
    assert_eq!(frame.candidates[0].text, first);
    assert_eq!(frame.candidates[1].text, "你好世界");
    assert!(frame.candidates[1].cloud);
    assert_eq!(frame.candidates[2].text, "拟好");
    assert!(frame.candidates[2].cloud);
    assert!(!frame.candidates[0].cloud);

    let frame = session.select(1);
    assert_eq!(frame.commit, "你好世界");
    assert!(frame.preedit.is_empty());
}

#[test]
fn tool_editors_pause_cloud_prediction() {
    let last = Arc::new(AtomicU64::new(0));
    let mut session = session(Arc::clone(&last));
    session.set_metered(false);
    session.push('n');
    session.push('i');
    assert_eq!(last.load(Ordering::SeqCst), 0);
    assert!(session.poll_prediction().is_none());
}
