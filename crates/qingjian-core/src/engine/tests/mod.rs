//! Engine 的测试：共用的样例词库、辅助函数与 mock 在这里，用例按主题分文件。

mod cloud;
mod correction;
mod emoji;
mod english;
mod learning;
mod lookup;
mod privacy;
mod shuangpin;

use std::collections::HashMap;

use std::sync::{Arc, Mutex};

use super::*;

use crate::candidate::{PartOfSpeech, Sense, Translation};

const SAMPLE: &str = "开发\tkai fa\t9000\n开发者\tkai fa zhe\t3000\n开饭\tkai fan\t800\n开放\tkai fang\t20000\n西安\txi an\t4000\n先\txian\t10000\n下\txia\t8000\n想\txiang\t9000\n开\tkai\t20000\n咖啡\tka fei\t5000\n";

fn engine() -> Engine {
    Engine::new(Dictionary::parse(SAMPLE).unwrap())
}

fn texts(input: &str) -> Vec<String> {
    let mut engine = engine();
    engine.set_input(input);
    engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .map(|c| c.text)
        .collect()
}

fn xiaohe() -> Engine {
    let mut engine = engine();
    engine.set_shuangpin(Some(Scheme::Xiaohe));
    engine
}

/// 与 [`Engine::take_raw`] 里的判断一致：原样上屏的串要不要记成英文词。
fn looks_like_english_word_in(engine: &Engine) -> bool {
    let raw = engine.composition().text();
    looks_like_english_word(raw, false) && engine.decode(raw).is_none_or(|d| !d.is_complete())
}

struct CountingLearner(HashMap<String, u32>);

impl Learner for CountingLearner {
    fn record(&mut self, candidate: &Candidate) {
        *self.0.entry(candidate.text.clone()).or_default() += 1;
    }

    fn weight(&self, text: &str) -> u32 {
        self.0.get(text).copied().unwrap_or(0)
    }

    fn record_choice(&mut self, input: &str, text: &str) {
        *self.0.entry(format!("{input}\t{text}")).or_default() += 1;
    }

    fn choice_weight(&self, input: &str, text: &str) -> u32 {
        self.0
            .get(&format!("{input}\t{text}"))
            .copied()
            .unwrap_or(0)
    }

    fn record_raw(&mut self, input: &str) {
        *self.0.entry(format!("{input}\t<raw>")).or_default() += 1;
    }

    fn record_typo(&mut self, typed: &str, intended: &str) {
        *self
            .0
            .entry(format!("typo\t{typed}\t{intended}"))
            .or_default() += 1;
    }

    fn unrecord_typo(&mut self, typed: &str, intended: &str) {
        if let Some(count) = self.0.get_mut(&format!("typo\t{typed}\t{intended}")) {
            *count = count.saturating_sub(1);
        }
    }

    fn typo_count(&self, typed: &str, intended: &str) -> u32 {
        self.0
            .get(&format!("typo\t{typed}\t{intended}"))
            .copied()
            .unwrap_or(0)
    }

    fn unrecord(&mut self, text: &str) {
        if let Some(count) = self.0.get_mut(text) {
            *count = count.saturating_sub(1);
        }
    }

    fn unrecord_choice(&mut self, input: &str, text: &str) {
        if let Some(count) = self.0.get_mut(&format!("{input}\t{text}")) {
            *count = count.saturating_sub(1);
        }
    }

    fn raw_count(&self, input: &str) -> u32 {
        self.0.get(&format!("{input}\t<raw>")).copied().unwrap_or(0)
    }
}

struct FixedTranslator;

impl Translator for FixedTranslator {
    fn language(&self) -> Language {
        Language::English
    }

    fn translate(&self, text: &str) -> Option<Translation> {
        (text == "开发").then(|| {
            Translation::new(
                Language::English,
                vec![Sense {
                    part_of_speech: Some(PartOfSpeech::Verb),
                    text: "develop".into(),
                    reading: None,
                    fresh: false,
                }],
            )
        })
    }
}

/// 把请求记下来、按序号原样回一条结果的假联想器。
struct EchoPredictor {
    submitted: std::rc::Rc<std::cell::RefCell<Vec<PredictionRequest>>>,
    replies: Vec<Prediction>,
    sentence: bool,
}

// 测试里单线程用，Rc 不 Send 但 trait 要求 Send；这里只在本线程访问
unsafe impl Send for EchoPredictor {}

impl Predictor for EchoPredictor {
    fn policy(&self) -> PredictionPolicy {
        PredictionPolicy {
            before: 4,
            after: 2,
            slots: 2,
            max_items: 2,
            sentence: self.sentence,
        }
    }

    fn submit(&mut self, request: PredictionRequest) {
        self.submitted.borrow_mut().push(request);
    }

    fn poll(&mut self) -> Option<Prediction> {
        self.replies.pop()
    }
}

fn cloud(text: &str, syllables: &[&str]) -> CloudWord {
    CloudWord {
        text: text.into(),
        syllables: syllables.iter().map(|s| (*s).to_owned()).collect(),
        reading: None,
    }
}

/// 记请求、按需吐结果的假释义兜底。
#[derive(Default)]
struct MemoryFiller {
    requested: Arc<Mutex<Vec<String>>>,
    ready: Arc<Mutex<Vec<FilledGloss>>>,
}

impl GlossFiller for MemoryFiller {
    fn request(&mut self, language: Language, word: &str) {
        assert_eq!(language, Language::English);
        self.requested.lock().unwrap().push(word.to_owned());
    }

    fn poll(&mut self) -> Vec<FilledGloss> {
        std::mem::take(&mut *self.ready.lock().unwrap())
    }
}

/// 会记住学到的释义的译者：随包只有 开发。
#[derive(Default)]
struct LearningTranslator(HashMap<String, Translation>);

impl Translator for LearningTranslator {
    fn language(&self) -> Language {
        Language::English
    }

    fn translate(&self, text: &str) -> Option<Translation> {
        self.0
            .get(text)
            .cloned()
            .or_else(|| FixedTranslator.translate(text))
    }

    fn learn(&mut self, word: &str, translation: Translation) {
        self.0.insert(word.to_owned(), translation);
    }
}

/// (看到轮次, 上屏次数, 用过次数)。
type VocabularyCounts = HashMap<(Language, String), (u32, u32, u32)>;

/// 记在内存里的词汇记录。
#[derive(Default)]
struct MemoryVocabulary(Arc<Mutex<VocabularyCounts>>);

impl VocabularyTracker for MemoryVocabulary {
    fn exposures(&self, language: Language, word: &str) -> u32 {
        self.0
            .lock()
            .unwrap()
            .get(&(language, word.to_owned()))
            .map_or(0, |entry| entry.0)
    }

    fn record_exposure(&mut self, language: Language, word: &str) {
        self.0
            .lock()
            .unwrap()
            .entry((language, word.to_owned()))
            .or_default()
            .0 += 1;
    }

    fn record_commit(&mut self, language: Language, word: &str, used: bool) {
        let mut map = self.0.lock().unwrap();
        let entry = map.entry((language, word.to_owned())).or_default();
        entry.1 += 1;
        entry.2 += u32::from(used);
    }
}

/// 把输入统计攒起来的假累计方。
struct MemoryMeter(Arc<Mutex<Vec<Usage>>>);

impl UsageMeter for MemoryMeter {
    fn record(&mut self, usage: Usage) {
        self.0.lock().unwrap().push(usage);
    }
}

/// 把输入日志条目攒起来的假落盘方。
struct MemoryLogger(Arc<Mutex<Vec<InputLogEntry>>>);

impl InputLogger for MemoryLogger {
    fn record(&mut self, entry: InputLogEntry) {
        self.0.lock().unwrap().push(entry);
    }
}

struct SentenceModel;

impl LanguageModel for SentenceModel {
    fn log_prob(&self, _previous: Option<&str>, word: &str) -> Option<f64> {
        match word {
            "开发" => Some(-5.0),
            "输入法" => Some(-6.0),
            "很" | "好" | "用" => Some(-7.0),
            "好用" => Some(-6.5),
            _ => None,
        }
    }
}

/// 记用户词与词转移的假学习器；`shared` 让测试从外面看到记了什么。
#[derive(Default)]
struct WordLearner {
    words: Vec<(String, Vec<String>)>,
    dictionary: Option<Dictionary>,
    ngram: sentence::UserNgram,
    shared: Arc<Mutex<(Vec<String>, sentence::UserNgram)>>,
    choices: HashMap<String, u32>,
}

impl Learner for WordLearner {
    fn record(&mut self, _candidate: &Candidate) {}

    fn record_choice(&mut self, input: &str, text: &str) {
        *self.choices.entry(format!("{input}\t{text}")).or_default() += 1;
    }

    fn choice_weight(&self, input: &str, text: &str) -> u32 {
        self.choices
            .get(&format!("{input}\t{text}"))
            .copied()
            .unwrap_or(0)
    }

    fn unrecord_choice(&mut self, input: &str, text: &str) {
        if let Some(count) = self.choices.get_mut(&format!("{input}\t{text}")) {
            *count = count.saturating_sub(1);
        }
    }

    fn weight(&self, text: &str) -> u32 {
        u32::from(self.words.iter().any(|(t, _)| t == text))
    }

    fn learn_word(&mut self, text: &str, syllables: &[String]) {
        self.words.push((text.to_owned(), syllables.to_vec()));
        let tsv: String = self
            .words
            .iter()
            .map(|(t, s)| format!("{t}\t{}\t100\n", s.join(" ")))
            .collect();
        self.dictionary = Some(Dictionary::parse(&tsv).unwrap());
        self.shared.lock().unwrap().0.push(text.to_owned());
    }

    fn user_words(&self) -> Option<&Dictionary> {
        self.dictionary.as_ref()
    }

    fn record_transition(&mut self, context: sentence::Context<'_>, word: &str, times: u32) {
        self.ngram.record_times(context, word, times);
        self.shared
            .lock()
            .unwrap()
            .1
            .record_times(context, word, times);
    }

    fn unrecord_transition(&mut self, context: sentence::Context<'_>, word: &str, times: u32) {
        self.ngram.unrecord(context, word, times);
        self.shared.lock().unwrap().1.unrecord(context, word, times);
    }

    fn user_ngram(&self) -> Option<&sentence::UserNgram> {
        Some(&self.ngram)
    }
}

/// 只认「做了 → 吧」与句首 把 的假模型。
struct BaModel;

impl LanguageModel for BaModel {
    fn log_prob(&self, previous: Option<&str>, word: &str) -> Option<f64> {
        match (previous, word) {
            (Some("做了"), "吧") => Some(-1.0),
            (Some("做了"), "把") => Some(-8.0),
            (None, "把") => Some(-3.0),
            (None, "吧") => Some(-7.0),
            _ => None,
        }
    }
}

fn texts_of(engine: &Engine) -> Vec<String> {
    engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .map(|c| c.text)
        .collect()
}
