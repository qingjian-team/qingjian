use std::time::{Duration, Instant};

use super::*;
use crate::sentence::SentenceWord;

/// 这几个测试里路径共同解释的那段按键。
const KEYS: &str = "kaifang";

/// 假打分器：偏爱某个文本，其余都给低分。
struct Prefers(&'static str);

impl SentenceScorer for Prefers {
    fn score(&self, _context: &str, _keys: &str, texts: &[&str]) -> Vec<f64> {
        texts
            .iter()
            .map(|t| if *t == self.0 { -1.0 } else { -20.0 })
            .collect()
    }
}

fn path(text: &str, score: f64) -> Conversion {
    Conversion {
        text: text.to_owned(),
        syllables: Vec::new(),
        words: vec![SentenceWord {
            text: text.to_owned(),
            syllables: Vec::new(),
            placeholder: false,
        }],
        score,
        static_score: score,
        penalty: 0.0,
    }
}

fn engine() -> Engine {
    Engine::new(Dictionary::parse("开发\tkai fa\t9000\n").unwrap())
}

fn texts(paths: &[Conversion]) -> Vec<&str> {
    paths.iter().map(|p| p.text.as_str()).collect()
}

#[test]
fn sync_scorer_reorders_paths_in_place() {
    let engine = engine().with_sentence_scorer(Box::new(Prefers("开放")), Some(0.5), None, None);
    let mut paths = vec![path("开饭", -10.0), path("开放", -11.0)];
    engine.rescore_paths(&mut paths, KEYS);
    assert_eq!(texts(&paths), ["开放", "开饭"]);
    // λ 0.5 排出来的名次：开饭 −10 + 0.5·(−20 + 10) = −15 落后于 开放 −11 + 0.5·(−1 + 11) = −6。
    // 分只用来排名次，不写回：`score` 还是静态尺度的路径分，跨读法的比较（拼写纠错、混输）靠它
    assert!((paths[0].score - -11.0).abs() < 1e-9);
    assert!((paths[1].score - -10.0).abs() < 1e-9);
    assert!(!engine.rescoring_pending());
}

#[test]
fn async_scorer_waits_for_the_shell_to_request_and_poll() {
    let mut engine =
        engine().with_async_sentence_scorer(Box::new(Prefers("开放")), Some(0.5), None, None);
    let mut paths = vec![path("开饭", -10.0), path("开放", -11.0)];
    engine.rescore_paths(&mut paths, KEYS);
    // 第一次：没分，顺序不动，记下要分的
    assert_eq!(texts(&paths), ["开饭", "开放"]);
    assert!(engine.rescoring_pending());
    assert!(engine.request_rescoring());
    assert!(!engine.rescoring_pending());
    let started = Instant::now();
    while !engine.poll_rescoring() {
        assert!(started.elapsed() < Duration::from_secs(5), "后台没回结果");
        std::thread::sleep(Duration::from_millis(5));
    }
    let mut paths = vec![path("开饭", -10.0), path("开放", -11.0)];
    engine.rescore_paths(&mut paths, KEYS);
    assert_eq!(texts(&paths), ["开放", "开饭"]);
    // 没有新的要打的就不发
    assert!(!engine.request_rescoring());
}

#[test]
fn a_changed_context_discards_the_cached_scores() {
    let mut engine =
        engine().with_async_sentence_scorer(Box::new(Prefers("开放")), Some(0.5), None, None);
    engine.history_mut().record("今天");
    let mut paths = vec![path("开饭", -10.0), path("开放", -11.0)];
    engine.rescore_paths(&mut paths, KEYS);
    assert!(engine.request_rescoring());
    let started = Instant::now();
    while !engine.poll_rescoring() {
        assert!(started.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(5));
    }
    // 上屏了别的字，前文变了：缓存作废，又得重新要
    engine.history_mut().record("很好");
    let mut paths = vec![path("开饭", -10.0), path("开放", -11.0)];
    engine.rescore_paths(&mut paths, KEYS);
    assert_eq!(texts(&paths), ["开饭", "开放"]);
    assert!(engine.rescoring_pending());
}

#[test]
fn the_shell_context_wins_over_session_history() {
    let mut engine = engine().with_sentence_scorer(Box::new(Prefers("开放")), None, None, Some(4));
    engine.history_mut().record("本会话上屏的历史");
    assert_eq!(engine.rescoring_context(), "屏的历史");
    engine.set_rescoring_context(Some("应用里光标前的文本".to_owned()));
    assert_eq!(engine.rescoring_context(), "前的文本");
    engine.set_rescoring_context(None);
    assert_eq!(engine.rescoring_context(), "屏的历史");
}

#[test]
fn changed_keys_discard_the_cached_scores() {
    let mut engine =
        engine().with_async_sentence_scorer(Box::new(Prefers("开放")), Some(0.5), None, None);
    let mut paths = vec![path("开饭", -10.0), path("开放", -11.0)];
    engine.rescore_paths(&mut paths, KEYS);
    assert!(engine.request_rescoring());
    let started = Instant::now();
    while !engine.poll_rescoring() {
        assert!(started.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(5));
    }
    // 同一段前文，但换了一段按键：P2C 的条件变了，缓存不能复用
    let mut paths = vec![path("开饭", -10.0), path("开放", -11.0)];
    engine.rescore_paths(&mut paths, "kaifan");
    assert_eq!(texts(&paths), ["开饭", "开放"]);
    assert!(engine.rescoring_pending());
}

/// 可控的假模型：每段生成都先告诉测试「开始了」（带上按键），再卡住等测试放行。
/// 后台线程按顺序处理任务：任务一旦开始就不会再被后来的请求合并掉；下一段开始时，上一段的结果一定已经发出来。
struct Gated {
    release: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,

    started: std::sync::Mutex<std::sync::mpsc::Sender<String>>,
}

impl SentenceScorer for Gated {
    fn score(&self, _context: &str, _keys: &str, texts: &[&str]) -> Vec<f64> {
        vec![-1.0; texts.len()]
    }

    fn generate(&self, keys: &str, _beam: usize, _max_chars: usize) -> Vec<String> {
        let _ = self.started.lock().unwrap().send(keys.to_owned());
        let _ = self.release.lock().unwrap().recv();
        vec![format!("{keys}的整句")]
    }
}

/// 打字比模型快时，上一段按键的结果会比这一段的先回来：它不算这一轮的结果，
/// 壳要接着等。否则壳当成已经拿到、停了轮询，晚几十毫秒到的这一段整句就没人取，空格上屏的还是旧首选
/// （macOS 上 `firefoxliulanqi` 上屏成「覅」，见 docs/notes/neural-rescoring.md）。
#[test]
fn an_earlier_keystrokes_result_does_not_end_the_round() {
    let (release, release_rx) = std::sync::mpsc::channel();
    let (started_tx, started) = std::sync::mpsc::channel();
    let mut engine = engine().with_async_sentence_scorer(
        Box::new(Gated {
            release: std::sync::Mutex::new(release_rx),
            started: std::sync::Mutex::new(started_tx),
        }),
        Some(0.5),
        None,
        None,
    );
    let wait = |keys: &str| assert_eq!(started.recv_timeout(Duration::from_secs(5)).unwrap(), keys);
    // 敲到 kaifa 停了一下：请求发出，后台开始算
    assert!(engine.generated_sentences("kaifa").is_empty());
    assert!(engine.request_rescoring());
    wait("kaifa");
    // 接着敲完 kaifang：又一次请求；kaifa 那段这时才算完
    assert!(engine.generated_sentences(KEYS).is_empty());
    assert!(engine.request_rescoring());
    release.send(()).unwrap();
    wait(KEYS);
    // 收到的只有 kaifa 的结果：存进缓存，但这一轮还没完
    assert!(!engine.poll_rescoring());
    assert_eq!(engine.generated_sentences("kaifa"), ["kaifa的整句"]);
    // kaifang 的到了，这一轮才算完
    release.send(()).unwrap();
    let started_at = Instant::now();
    while !engine.poll_rescoring() {
        assert!(
            started_at.elapsed() < Duration::from_secs(5),
            "后台没回结果"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(engine.generated_sentences(KEYS), ["kaifang的整句"]);
}
