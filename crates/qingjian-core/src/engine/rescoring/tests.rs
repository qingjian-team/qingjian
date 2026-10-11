use std::time::{Duration, Instant};

use qingjian_dictionary::Dictionary;

use super::*;
use crate::sentence::{SentenceScorer, SentenceWord, UserNgram};

#[path = "tests/learned.rs"]
mod learned;

use learned::Learned;

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

#[test]
fn sentence_middle_alternatives_survive_and_mode_comparisons_preserve_scores() {
    const TEXT: &str = "来试一试这个新输入法好不好用";
    const KEYS: &str = "laishiyishizhegexinshurufahaobuhaoyong";
    for english in [false, true] {
        for asynchronous in [false, true] {
            let mut engine = Engine::new(
                Dictionary::parse(
                    "来\tlai\t100\n是\tshi\t1000\n试\tshi\t100\n一\tyi\t100\n\
                 这个\tzhe ge\t100\n新\txin\t100\n输入法\tshu ru fa\t100\n\
                 好\thao\t100\n不\tbu\t100\n用\tyong\t100\n",
                )
                .unwrap(),
            );
            if english {
                engine = engine.with_english(
                    qingjian_dictionary::WordList::parse("Yong\tyong\t2000\n").unwrap(),
                );
            }
            // 模型加载前格子已被查过；别处常选的「是 / 一」不能挤掉本句的「试一试」。
            engine = engine.with_learner(Box::new(Learned::biased()));
            engine.set_input(KEYS);
            engine.query().unwrap();
            let scorer = Box::new(Prefers(TEXT));
            let mut engine = if asynchronous {
                engine.with_async_sentence_scorer(scorer, Some(1.0), None, None)
            } else {
                engine.with_sentence_scorer(scorer, Some(1.0), None, None)
            };
            engine.set_input(KEYS);
            engine.query().unwrap();
            if asynchronous {
                assert!(engine.request_rescoring());
                let start = Instant::now();
                while !engine.poll_rescoring() {
                    assert!(start.elapsed() < Duration::from_secs(5));
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
            let query = engine.query().unwrap();
            assert_eq!(
                query.candidates.items[0].text, TEXT,
                "english={english}, async={asynchronous}"
            );
            assert_eq!(query.candidates.items[0].syllables.concat(), KEYS);
            assert!(!engine.request_rescoring());
            assert_eq!(engine.commit(&query.candidates.items[0]), TEXT);
            assert!(engine.composition().is_empty());
        }
    }
}

#[test]
fn sync_and_async_rescorers_compare_all_legal_segmentations() {
    // 静态频率偏爱贪心切分；本地模型应能在同一批路径中选中另一种合法切分。
    for asynchronous in [false, true] {
        let engine = Engine::new(
            Dictionary::parse(
                "先\txian\t100\n想\txiang\t1000\n给\tgei\t100\n欸\tei\t1000\n\
             我\two\t1000\n看看\tkan kan\t1000\n",
            )
            .unwrap(),
        );
        let scorer = Box::new(Prefers("先给我看看"));
        let mut engine = if asynchronous {
            engine.with_async_sentence_scorer(scorer, Some(1.0), Some(100.0), None)
        } else {
            engine.with_sentence_scorer(scorer, Some(1.0), Some(100.0), None)
        };
        engine.set_input("xiangeiwokankan");
        if asynchronous {
            assert_eq!(
                engine.query().unwrap().candidates.items[0].text,
                "想欸我看看"
            );
            assert!(engine.request_rescoring());
            let start = Instant::now();
            while !engine.poll_rescoring() {
                assert!(start.elapsed() < Duration::from_secs(5));
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        let query = engine.query().unwrap();
        assert_eq!(query.candidates.items[0].text, "先给我看看");
        assert_eq!(query.marked_text(), "xian'gei'wo'kan'kan");
        assert!(!engine.request_rescoring());
    }
}

#[test]
fn mixed_sentence_rescoring_preserves_the_winning_head_scores() {
    let dictionary = Dictionary::parse(
        "先\txian\t10000\n西安\txi an\t100\n给\tgei\t10000\n我\two\t10000\n\
         看看\tkan kan\t10000\n茹\tru\t100\n斯\tsi\t100\n特\tte\t100\n",
    )
    .unwrap();
    let mut engine = Engine::new(dictionary)
        .with_english(qingjian_dictionary::WordList::parse("rust\trust\t3740\n").unwrap())
        .with_async_sentence_scorer(Box::new(Prefers("西安给我看看")), Some(1.0), None, None);
    engine.set_input("xiangeiwokankanrust");
    let first = engine.query().unwrap();
    assert_eq!(first.candidates.items[0].text, "先给我看看rust");
    assert!(engine.request_rescoring());
    let start = Instant::now();
    while !engine.poll_rescoring() {
        assert!(start.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(5));
    }
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "西安给我看看rust");
    assert!(!engine.request_rescoring());
    assert_eq!(
        engine.commit(&query.candidates.items[0]),
        "西安给我看看rust"
    );
    assert!(engine.composition().is_empty());
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
fn rescoring_preferences_apply_only_to_the_same_full_input() {
    for input in [KEYS, "shiyishi"] {
        let engine = engine()
            .with_learner(Box::new(Learned {
                ngram: UserNgram::default(),
                chosen: Some((input, "开放")),
            }))
            .with_sentence_scorer(Box::new(Prefers("neither")), Some(0.5), None, None);
        let mut paths = vec![path("开饭", -10.0), path("开放", -11.0)];
        engine.rescore_paths(&mut paths, KEYS);
        assert_eq!(paths[0].text, if input == KEYS { "开放" } else { "开饭" });
        assert_eq!(
            paths.iter().find(|p| p.text == "开放").unwrap().score,
            -11.0
        );
    }
}

#[test]
fn rescorer_preserves_full_pinyin_in_plain_and_mixed_sentences() {
    let dictionary = Dictionary::parse(
        "很\then\t100\n简短\tjian duan\t1\n简单\tjian dan\t10000000\n\
         的\tde\t100\n技术\tji shu\t100\n",
    )
    .unwrap();
    let mut engine = Engine::new(dictionary)
        .with_english(qingjian_dictionary::WordList::parse("rust\trust\t3740\n").unwrap())
        .with_sentence_scorer(Box::new(Prefers("很简单的技术")), Some(1.0), None, None);
    for (input, expected) in [
        ("henjianduandejishu", "很简短的技术"),
        ("henjianduandejishurust", "很简短的技术rust"),
    ] {
        engine.set_input(input);
        let query = engine.query().unwrap();
        assert_eq!(query.candidates.items[0].text, expected);
        assert_eq!(query.candidates.items[0].syllables.concat(), input);
    }
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
