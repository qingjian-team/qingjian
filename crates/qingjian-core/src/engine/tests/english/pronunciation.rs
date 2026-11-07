//! 英文候选的音标（`[general] show_english_pronunciation`）。
//!
//! 三个要点：中文候选不受影响、关着时与以前完全一样、查不到音标的词不出错也不显示。

use super::*;

/// 一份最小音标表：只有 hello 与 cargo 有音标。
fn pronunciations() -> WordList {
    WordList::parse_pronunciations("hello\t/həˈloʊ/\ncargo\t/ˈkɑrˌɡoʊ/\n").unwrap()
}

/// 测试用的对照译文：`词 → 一条释义`，只做查表，不起真释义表。
struct StubTranslator {
    language: Language,
    entries: Vec<(&'static str, &'static str)>,
}

impl StubTranslator {
    fn new(language: Language, entries: Vec<(&'static str, &'static str)>) -> Self {
        Self { language, entries }
    }
}

impl Translator for StubTranslator {
    fn language(&self) -> Language {
        self.language
    }

    fn translate(&self, text: &str) -> Option<Translation> {
        let (_, gloss) = self.entries.iter().find(|(word, _)| *word == text)?;
        Some(Translation::new(
            self.language,
            vec![Sense {
                part_of_speech: None,
                text: (*gloss).to_owned(),
                reading: None,
                pronunciation: None,
                fresh: false,
            }],
        ))
    }
}

fn english_engine() -> Engine {
    let words =
        WordList::parse("hello\thello\t4720\ncargo\tcargo\t5000\ncompa\tcompa\t3000\n").unwrap();
    let mut engine = engine().with_english(words);
    engine.set_english_pronunciations(Some(pronunciations()));
    engine
}

fn first_english(engine: &mut Engine, input: &str) -> Candidate {
    engine.set_input(input);
    engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.kind == CandidateKind::English)
        .expect("应该有英文候选")
}

#[test]
fn off_by_default_so_english_candidates_are_unchanged() {
    let mut engine = english_engine();
    // 没调 set_show_english_pronunciation，就是缺省的关
    assert!(!engine.show_english_pronunciation());
    let candidate = first_english(&mut engine, "hello");
    assert_eq!(candidate.text, "hello");
    assert_eq!(candidate.reading, None, "关着时不该有音标");
}

#[test]
fn on_fills_the_reading_with_ipa() {
    let mut engine = english_engine();
    engine.set_show_english_pronunciation(true);
    let candidate = first_english(&mut engine, "hello");
    assert_eq!(candidate.reading.as_deref(), Some("/həˈloʊ/"));
}

/// 查不到音标的英文词照常出候选，只是没有 reading——不能报错、也不能整条候选消失。
#[test]
fn a_word_without_ipa_still_shows_without_reading() {
    let mut engine = english_engine();
    engine.set_show_english_pronunciation(true);
    // compa 在词表里，但音标表里没有
    let candidate = first_english(&mut engine, "compa");
    assert_eq!(candidate.text, "compa");
    assert_eq!(candidate.reading, None);
}

/// 大写开头的词也查得到：查询按小写码走（`Cargo` 与 `cargo` 同一份数据）。
#[test]
fn lookup_is_case_insensitive() {
    let mut engine = english_engine();
    engine.set_show_english_pronunciation(true);
    let candidate = first_english(&mut engine, "cargo");
    assert_eq!(candidate.reading.as_deref(), Some("/ˈkɑrˌɡoʊ/"));
}

/// 中文候选从来不带 reading，开关不该把它们变成有音标的。
#[test]
fn chinese_candidates_never_get_a_reading() {
    let mut engine = english_engine();
    engine.set_show_english_pronunciation(true);
    // kaifa 在 SAMPLE 词库里是「开发 / 开放 / 开饭」，会出中文候选
    engine.set_input("kaifa");
    let chinese: Vec<Candidate> = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .filter(|c| c.kind == CandidateKind::Chinese)
        .collect();
    assert!(!chinese.is_empty(), "应该有中文候选");
    for candidate in &chinese {
        assert_eq!(
            candidate.reading, None,
            "中文候选 {} 不该带音标",
            candidate.text
        );
    }
}

/// 没有音标表（随包数据里没有这个文件）时开着开关也不出错。
#[test]
fn no_table_means_no_reading_and_never_panics() {
    let words = WordList::parse("hello\thello\t4720\n").unwrap();
    let mut engine = engine().with_english(words);
    engine.set_show_english_pronunciation(true);
    let candidate = first_english(&mut engine, "hello");
    assert_eq!(candidate.reading, None);
}

/// 英文模式（`query_english`）是另一条生成路径：候选全部来自 `english::suggest`，
/// 与中英混输的 `insert_english` 不共用代码，漏一处就没有音标。
#[test]
fn english_mode_candidates_carry_ipa_too() {
    let mut engine = english_engine();
    engine.set_show_english_pronunciation(true);
    engine.set_english_mode(true);
    engine.set_input("hello");
    let english: Vec<Candidate> = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .filter(|c| c.kind == CandidateKind::English)
        .collect();
    assert!(!english.is_empty(), "英文模式应该有英文候选");
    let hello = english
        .iter()
        .find(|c| c.text == "hello")
        .expect("应该有 hello");
    assert_eq!(hello.reading.as_deref(), Some("/həˈloʊ/"));
}

/// 英文模式下关着开关也一样没有音标。
#[test]
fn english_mode_respects_the_switch() {
    let mut engine = english_engine();
    engine.set_english_mode(true);
    engine.set_input("hello");
    // 先取出所有英文候选，避免同时可变借用 engine
    let english: Vec<Candidate> = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .filter(|c| c.kind == CandidateKind::English)
        .collect();
    assert!(!english.is_empty(), "英文模式应该有英文候选");
    for candidate in &english {
        assert_eq!(candidate.reading, None, "{} 不该带音标", candidate.text);
    }
}

/// 中文候选的**译文**是英文词时也注音标（`开发` → `hello /həˈloʊ/`），
/// 这是 `Sense::pronunciation`，与候选自己的 `reading` 是两回事。
///
/// 中文候选的译文走 `translator`（学习语言的释义表），不是英文候选那条 `english_translator`。
#[test]
fn chinese_translations_carry_ipa_on_the_sense() {
    // SAMPLE 词库里 kaifa → 开发 / 开放 / 开饭
    let mut engine = engine();
    engine.set_translator(Box::new(StubTranslator::new(
        Language::English,
        vec![("开发", "hello")],
    )));
    engine.set_english_pronunciations(Some(pronunciations()));
    engine.set_show_english_pronunciation(true);

    engine.set_input("kaifa");
    let mut query = engine.query().unwrap();
    engine.annotate(&mut query.candidates);

    let hello = query
        .candidates
        .items
        .iter()
        .find(|c| c.text == "开发")
        .and_then(|c| c.translation.as_ref())
        .and_then(|t| t.senses().first())
        .expect("开发 应该有英文译文");
    assert_eq!(hello.text, "hello");
    assert_eq!(hello.pronunciation.as_deref(), Some("/həˈloʊ/"));
}

/// 关着开关时译文的 `pronunciation` 保持 `None`。
#[test]
fn translation_ipa_needs_the_switch() {
    let mut engine = engine();
    engine.set_translator(Box::new(StubTranslator::new(
        Language::English,
        vec![("开发", "hello")],
    )));
    engine.set_english_pronunciations(Some(pronunciations()));

    engine.set_input("kaifa");
    let mut query = engine.query().unwrap();
    engine.annotate(&mut query.candidates);

    for candidate in &query.candidates.items {
        if let Some(translation) = &candidate.translation {
            for sense in translation.senses() {
                assert_eq!(sense.pronunciation, None, "{} 不该带音标", sense.text);
            }
        }
    }
}

/// 日语译词走假名（`reading`），不该被当英文填 `pronunciation`。
#[test]
fn japanese_translations_do_not_get_ipa() {
    let mut engine = engine();
    engine.set_translator(Box::new(StubTranslator::new(
        Language::Japanese,
        vec![("开发", "こんにちは")],
    )));
    engine.set_english_pronunciations(Some(pronunciations()));
    engine.set_show_english_pronunciation(true);

    engine.set_input("kaifa");
    let mut query = engine.query().unwrap();
    engine.annotate(&mut query.candidates);

    for candidate in &query.candidates.items {
        if let Some(translation) = &candidate.translation {
            for sense in translation.senses() {
                assert_eq!(
                    sense.pronunciation, None,
                    "日语译文 {} 不该有 IPA",
                    sense.text
                );
            }
        }
    }
}
