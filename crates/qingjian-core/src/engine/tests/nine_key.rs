//! 九宫格：数字串出候选、整句、拼音列、选定拼音、逐词上屏。

use super::*;

fn nine_key() -> Engine {
    let mut engine = engine();
    engine.set_nine_key(true);
    engine
}

fn query(engine: &mut Engine, keys: &str) -> Query {
    engine.set_input(keys);
    engine.query().unwrap()
}

fn texts_of(query: &Query) -> Vec<&str> {
    query
        .candidates
        .items
        .iter()
        .map(|c| c.text.as_str())
        .collect()
}

#[test]
fn digits_find_words_across_readings() {
    // 52432：kai fa / kai fb…，词库里读得通的只有 开发
    let mut engine = nine_key();
    let query = query(&mut engine, "52432");
    assert_eq!(texts_of(&query)[0], "开发");
    assert_eq!(query.marked_text(), "kai'fa");
}

#[test]
fn unfinished_last_syllable_reads_as_prefix() {
    // 5243：kai f… → 开发、开放、开饭 都该在
    let mut engine = nine_key();
    let query = query(&mut engine, "5243");
    let texts = texts_of(&query);
    assert!(texts.contains(&"开发") && texts.contains(&"开放") && texts.contains(&"开饭"));
}

#[test]
fn prefix_words_allow_word_by_word_commit() {
    // 52432943：开发者（kai fa zhe）的码；先选 开发，剩下 943 接着组句（开发 先留着，整段选完才上屏）
    let mut engine = nine_key();
    let query = query(&mut engine, "52432943");
    let kaifa = query
        .candidates
        .items
        .iter()
        .find(|c| c.text == "开发")
        .cloned()
        .unwrap();
    assert_eq!(engine.commit(&kaifa), "");
    assert_eq!(engine.held_text(), "开发");
    assert_eq!(engine.composition().text(), "943");
}

#[test]
fn committing_consumes_the_separator() {
    let mut engine = nine_key();
    let query = query(&mut engine, "94'26");
    let xian = query
        .candidates
        .items
        .iter()
        .find(|c| c.text == "西安")
        .cloned()
        .unwrap();
    engine.commit(&xian);
    assert!(engine.composition().is_empty());
}

#[test]
fn pinyin_choices_list_first_open_position() {
    let mut engine = nine_key();
    let query = query(&mut engine, "52432");
    // 524：kai 之外还有 lai 这个同码音节；带候选的 kai 排最前
    assert_eq!(query.pinyin_choices[0], "kai");
    assert!(query.pinyin_choices.iter().any(|choice| choice == "lai"));
}

#[test]
fn pinning_replaces_digits_with_letters() {
    let mut engine = nine_key();
    engine.set_input("52432");
    assert!(engine.pin_nine_key("kai"));
    assert_eq!(engine.composition().text(), "kai'32");
    let query = engine.query().unwrap();
    assert_eq!(texts_of(&query)[0], "开发");
    // 第一个位置定了，拼音列换成第二个位置的写法
    assert!(query.pinyin_choices.iter().any(|choice| choice == "fa"));
    assert!(!engine.pin_nine_key("zzz"));
}

#[test]
fn choice_learning_is_keyed_by_pinyin() {
    let mut engine = engine().with_learner(Box::new(CountingLearner(HashMap::new())));
    engine.set_nine_key(true);
    engine.set_input("52432");
    let candidate = engine.query().unwrap().candidates.items[0].clone();
    let (consumed, input) = engine.consumed_by(&candidate);
    assert_eq!((consumed, input.as_str()), (5, "kaifa"));
}

#[test]
fn calendar_shortcuts_need_pinned_letters() {
    let shortcut = |query: &Query| {
        query
            .candidates
            .items
            .iter()
            .any(|c| c.kind == CandidateKind::Shortcut)
    };
    // 数字串 77 不认 rq
    let mut engine = nine_key();
    assert!(!shortcut(&query(&mut engine, "77")));
    // 拼音列把 r、q 都定下来了才出日期
    let mut engine = nine_key();
    assert!(shortcut(&query(&mut engine, "r'q")));
    // 全拼的 时间（744526）跟着现在的时间
    let mut engine = Engine::new(Dictionary::parse(KEYWORDS).unwrap());
    engine.set_nine_key(true);
    let query = query(&mut engine, "744526");
    let texts = texts_of(&query);
    let at = texts.iter().position(|t| *t == "时间").unwrap();
    assert!(texts[at + 1].contains(':'), "{texts:?}");
}
