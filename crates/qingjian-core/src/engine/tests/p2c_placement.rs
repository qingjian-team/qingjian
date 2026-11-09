//! 模型直接按按键生成的整句候选排在哪：接了生成器时够长的干净拼音以生成为主，短的、没打完、纠正过的词图在前。

use super::*;
use crate::sentence::SentenceScorer;

/// 假模型：什么按键都生成同样几句；`generates` 决定它算不算生成器。
struct Fixed {
    texts: &'static [&'static str],

    generates: bool,
}

impl SentenceScorer for Fixed {
    fn score(&self, _context: &str, _keys: &str, texts: &[&str]) -> Vec<f64> {
        vec![-1.0; texts.len()]
    }

    fn generate(&self, _keys: &str, _beam: usize, _max_chars: usize) -> Vec<String> {
        self.texts.iter().map(|t| (*t).to_owned()).collect()
    }

    fn generates(&self) -> bool {
        self.generates
    }
}

fn engine(generates: bool) -> Engine {
    engine_generating(&["你好视界"], generates)
}

fn engine_generating(texts: &'static [&'static str], generates: bool) -> Engine {
    let dictionary = Dictionary::parse(
        "你好\tni hao\t900000\n世界\tshi jie\t500000\n是\tshi\t900000\n节\tjie\t100000\n",
    )
    .unwrap();
    Engine::new(dictionary).with_sentence_scorer(
        Box::new(Fixed { texts, generates }),
        None,
        None,
        None,
    )
}

fn top(engine: &mut Engine, input: &str) -> Vec<(String, CandidateKind)> {
    engine.set_input(input);
    engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .take(2)
        .map(|c| (c.text, c.kind))
        .collect()
}

/// 够长（6 个音节起）的干净拼音生成的排第一。
#[test]
fn a_generator_leads_on_long_clean_pinyin() {
    let mut engine = engine_generating(&["你好视界你好"], true);
    assert_eq!(
        top(&mut engine, "nihaoshijienihao"),
        [
            ("你好视界你好".to_owned(), CandidateKind::Generated),
            ("你好世界你好".to_owned(), CandidateKind::Sentence),
        ]
    );
}

/// 短的干净拼音词图在前（个人习惯在词图里），生成的排第二。
#[test]
fn the_lattice_leads_on_short_clean_pinyin() {
    let mut engine = engine(true);
    assert_eq!(
        top(&mut engine, "nihaoshijie"),
        [
            ("你好世界".to_owned(), CandidateKind::Sentence),
            ("你好视界".to_owned(), CandidateKind::Generated),
        ]
    );
}

/// 生成的第一句就是词图的整句时用词图那条（带读音），名次照生成的排，后面的生成句不会顶上来。
#[test]
fn a_generated_sentence_the_lattice_also_found_keeps_its_rank() {
    let mut engine = engine_generating(&["你好世界你好", "你好视界你好"], true);
    assert_eq!(
        top(&mut engine, "nihaoshijienihao"),
        [
            ("你好世界你好".to_owned(), CandidateKind::Sentence),
            ("你好视界你好".to_owned(), CandidateKind::Generated),
        ]
    );
}

/// 末尾音节没打完时生成器读不准，词图（带补全）排第一，生成的跟在后面。
#[test]
fn the_lattice_leads_while_the_last_syllable_is_unfinished() {
    let mut engine = engine(true);
    assert_eq!(
        top(&mut engine, "nihaoshij"),
        [
            ("你好世界".to_owned(), CandidateKind::Sentence),
            ("你好视界".to_owned(), CandidateKind::Generated),
        ]
    );
}

/// 不是生成器的模型（含章·知微）在干净的拼音上不生成，只重排词图的路径。
#[test]
fn a_reranker_does_not_generate_on_clean_pinyin() {
    let mut engine = engine(false);
    engine.set_input("nihaoshijie");
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "你好世界");
    assert!(
        query
            .candidates
            .items
            .iter()
            .all(|c| c.kind != CandidateKind::Generated)
    );
}
