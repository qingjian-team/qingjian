//! 形码（五笔）：编码直接查码表，不走拼音的切分与整句。

use qingjian_dictionary::CodeTable;

use super::*;

/// 五笔 86 的一小段：一级简码 V 是 发，`ggll` 是 一。
/// `甲` 的编码故意取成 `kai`、`乙丙` 取成 `kaik`——与拼音的 开 同形，用来验混输时的排序。
pub(super) const CODES: &str = "一\tggll\t90000\n发\tv\t30000\n到\tgc\t20000\n来\tgo\t15000\n开\tga\t5000\n开发\tgant\t900\n甲\tkai\t1000\n乙丙\tkaik\t800\n";

/// 只用形码：拼音侧关掉，候选只从码表来。
pub(super) fn wubi() -> Engine {
    let mut engine = engine();
    engine.set_code_scheme(Some(CodeScheme::new(
        "wubi",
        CodeTable::parse(CODES).unwrap(),
    )));
    engine.set_phonetic(false);
    engine
}

/// 混输：形码与拼音两边都出候选。
fn mixed() -> Engine {
    let mut engine = engine();
    engine.set_code_scheme(Some(CodeScheme::new(
        "wubi",
        CodeTable::parse(CODES).unwrap(),
    )));
    engine
}

fn code_texts(engine: &mut Engine, input: &str) -> Vec<String> {
    engine.set_input(input);
    texts_of(engine)
}

#[test]
fn looks_up_by_code_prefix_and_prefers_the_finished_code() {
    let mut engine = wubi();
    // 编码打全的 开 排在同前缀的 开发 前面
    assert_eq!(code_texts(&mut engine, "ga"), ["开", "开发"]);
    // 没打全时只按词频
    assert_eq!(
        code_texts(&mut engine, "g"),
        ["一", "到", "来", "开", "开发"]
    );
    // preedit 原样显示敲的编码，没有拼音切分
    engine.set_input("ga");
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "ga");
    assert_eq!(query.tail, "ga");
    assert!(query.segmentations.is_empty());
    assert!(!query.decoded_keys);
    assert!(
        query
            .candidates
            .items
            .iter()
            .all(|c| c.syllables.is_empty())
    );
}

#[test]
fn commits_the_whole_code_and_empties_the_buffer() {
    let mut engine = wubi();
    engine.set_input("ga");
    let kai = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(kai.kind, CandidateKind::Code);
    assert_eq!(engine.commit(&kai), "开");
    assert!(engine.composition().is_empty());
}

#[test]
fn takes_no_part_in_the_pinyin_machinery() {
    let mut engine = wubi();
    // `v` 是字根键，不当表达式模式的前缀
    assert_eq!(code_texts(&mut engine, "v"), ["发"]);
    // 快捷候选按拼音键认，形码下不出
    assert!(code_texts(&mut engine, "rq").is_empty());
    // 模糊音不参与：`ga` 不会命中并不存在的 `ka`
    engine.set_fuzzy(FuzzyRules::ALL);
    assert_eq!(code_texts(&mut engine, "ga"), ["开", "开发"]);
}

#[test]
fn a_word_chosen_under_a_code_rises_next_time() {
    let mut engine = wubi().with_learner(Box::new(WordLearner::default()));
    engine.set_input("g");
    let kaifa = engine
        .query()
        .unwrap()
        .candidates
        .items
        .last()
        .cloned()
        .unwrap();
    assert_eq!(kaifa.text, "开发");
    engine.commit(&kaifa);
    engine.set_input("g");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "开发");
}

#[test]
fn code_candidates_get_translations_like_pinyin_ones() {
    let mut engine = wubi().with_translator(Box::new(FixedTranslator));
    engine.set_input("gant");
    let mut query = engine.query().unwrap();
    engine.annotate(&mut query.candidates);
    let kaifa = query
        .candidates
        .items
        .iter()
        .find(|c| c.text == "开发")
        .unwrap();
    let senses = kaifa.translation.as_ref().unwrap().senses();
    assert_eq!(senses[0].text, "develop");
}

#[test]
fn code_commits_still_feed_translations_vocabulary_and_usage() {
    // 形码换的是「怎么按键出候选」，不是「上屏之后算什么」：释义标注、生词判定、词汇记录与输入统计
    // 都按上屏的词工作，与方案无关。这条用例把这句话钉住。
    let usage = Arc::new(Mutex::new(Vec::new()));
    let vocabulary = Arc::new(Mutex::new(VocabularyCounts::default()));
    let mut engine = wubi()
        .with_translator(Box::new(FixedTranslator))
        .with_usage_meter(Box::new(MemoryMeter(usage.clone())))
        .with_vocabulary_tracker(Box::new(MemoryVocabulary(vocabulary.clone())));

    engine.set_input("gant");
    let mut query = engine.query().unwrap();
    engine.annotate(&mut query.candidates);
    let kaifa = query.candidates.items[0].clone();
    assert_eq!(kaifa.text, "开发");
    // 释义照常标注；这条译词一个轮次都没见过，标成生词
    let sense = &kaifa.translation.as_ref().unwrap().senses()[0];
    assert_eq!(sense.text, "develop");
    assert!(sense.fresh);

    engine.commit(&kaifa);
    // 输入统计：一个中文词、两个汉字（形码一次上屏就是一个词）
    let usage = usage.lock().unwrap();
    assert_eq!(usage.len(), 1);
    assert_eq!((usage[0].words, usage[0].hanzi), (1, 2));
    // 词汇记录：上屏带译词的中文候选，那条译词记「上屏过」；没按修饰键打出来过，不是「用过」
    let counts = vocabulary.lock().unwrap();
    let entry = counts
        .get(&(Language::English, "develop".to_owned()))
        .expect("译词应当进了词汇记录");
    assert_eq!((entry.1, entry.2), (1, 0));
}

#[test]
fn mixed_input_orders_finished_codes_then_pinyin_then_code_prefixes() {
    let mut engine = mixed();
    // `kai`：编码打全的 甲 在最前，拼音的 开 一族居中，只命中前缀的 乙丙（`kaik`）垫后
    let texts = code_texts(&mut engine, "kai");
    assert_eq!(texts.first().map(String::as_str), Some("甲"));
    let position = |text: &str| texts.iter().position(|t| t == text).unwrap();
    assert!(position("开") < position("乙丙"));
    // `ka`：没有打全的编码，拼音的首选不被前缀命中的形码词顶掉
    let texts = code_texts(&mut engine, "ka");
    assert_ne!(texts.first().map(String::as_str), Some("甲"));
    assert!(texts.contains(&"甲".to_owned()));
}

#[test]
fn mixed_input_keeps_the_shifted_mode_letters() {
    // 混输下小写 v / u 是字根键，表达式与问字同双拼一样改用大写进
    let mut engine = mixed();
    assert!(engine.takes_mode_letter('V') && engine.takes_mode_letter('U'));
    engine.set_input("V1+2");
    assert!(engine.expression_mode());
    engine.set_input("v");
    assert!(!engine.expression_mode());
    assert_eq!(
        code_texts(&mut engine, "v").first().map(String::as_str),
        Some("发")
    );
}

#[test]
fn turning_the_pinyin_side_off_leaves_only_the_code_table() {
    let mut engine = wubi();
    assert_eq!(code_texts(&mut engine, "kai"), ["甲", "乙丙"]);
}

#[test]
fn a_longer_input_drops_the_code_side_by_itself() {
    // 五笔码最长 4 位：从第 5 个字母起形码查不到东西，混输下自然只剩拼音
    let mut engine = mixed();
    let texts = code_texts(&mut engine, "kaifa");
    assert!(texts.contains(&"开发".to_owned()));
    assert!(!texts.contains(&"甲".to_owned()));
}

#[test]
fn mixed_input_falls_back_to_the_code_side_when_pinyin_cannot_read_it() {
    // `ggll` 切不成音节，但码表有 一：混输不该因为拼音读不出来就整个失败
    let mut engine = mixed();
    assert_eq!(code_texts(&mut engine, "ggll"), ["一"]);
}

#[test]
fn keys_outside_the_code_alphabet_fall_back_to_raw() {
    let mut engine = wubi();
    engine.set_input("no-way");
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "no-way");
    assert_eq!(query.candidates.items[0].kind, CandidateKind::English);
}

#[test]
fn letters_never_start_a_mode_under_wubi() {
    // 形码下每个字母都是字根键，连大写也让位（双拼那套 Shift+V / Shift+U 在这里不适用）：
    // 字母一律进缓冲区，表达式与问字模式只剩 `?` 这一个人口。
    let mut engine = wubi();
    assert!(!engine.takes_mode_letter('V') && !engine.takes_mode_letter('U'));
    engine.set_input("V1+2");
    assert!(!engine.expression_mode());
    engine.set_input("Usangemu");
    assert!(!engine.question_mode());
    engine.set_mode_keys(ModeKeys {
        question_mark: true,
        shortcut: 'i',
        ..ModeKeys::default()
    });
    engine.set_input("?nihao");
    assert!(engine.question_mode());
}

#[test]
fn switching_back_to_pinyin_drops_the_code_table() {
    let mut engine = wubi();
    engine.set_code_scheme(None);
    assert!(!engine.is_code_mode());
    engine.set_input("kaifa");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "开发");
}

#[test]
fn mixed_input_works_with_shuangpin() {
    // 拼音侧换成双拼也是混输：编码照查码表，双拼的键按双拼切
    let mut engine = mixed();
    engine.set_shuangpin(Some(crate::shuangpin::Scheme::Xiaohe));
    assert_eq!(code_texts(&mut engine, "gant"), ["开发"]);
    assert_eq!(code_texts(&mut engine, "ggll"), ["一"]);
    assert_eq!(
        code_texts(&mut engine, "kdfa").first().map(String::as_str),
        Some("开发")
    );
    assert_eq!(code_texts(&mut engine, "xm"), ["先"]);
}
