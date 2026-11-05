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
fn pinyin_choices_also_offer_shorter_syllables_and_letters() {
    // 643226：最优切法开头是 nie / mie，还要能选 ni、mi 与单个字母，不然 你 这类字选不到
    let mut engine = nine_key();
    let query = query(&mut engine, "643226");
    let choices = &query.pinyin_choices;
    for wanted in ["nie", "mie", "ni", "mi", "n", "m", "o"] {
        assert!(
            choices.iter().any(|c| c == wanted),
            "{wanted} 不在 {choices:?}"
        );
    }
    let at = |s: &str| choices.iter().position(|c| c == s).unwrap();
    assert!(at("nie") < at("ni") && at("ni") < at("n"));
}

#[test]
fn shorter_syllables_follow_the_candidates_that_read_them() {
    // 6433：最优切法是 nie'e，你的 读 ni de；同样长的 ni 要排在没有候选的 mi 前面
    let mut engine = Engine::new(Dictionary::parse("你的\tni de\t9000\n").unwrap());
    engine.set_nine_key(true);
    let query = query(&mut engine, "6433");
    let choices = &query.pinyin_choices;
    let at = |s: &str| choices.iter().position(|c| c == s).unwrap();
    assert!(at("nie") < 2 && at("mie") < 2, "{choices:?}");
    assert!(at("ni") < at("mi"), "{choices:?}");
}

#[test]
fn pinning_a_shorter_syllable_splits_after_it() {
    let mut engine = nine_key();
    engine.set_input("643226");
    assert!(engine.pin_nine_key("ni"));
    assert_eq!(engine.composition().text(), "ni'3226");
    engine.set_input("643226");
    assert!(engine.pin_nine_key("n"));
    assert_eq!(engine.composition().text(), "n'43226");
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

#[test]
fn interjection_spellings_are_in_the_pinyin_column() {
    let mut engine = nine_key();
    for (keys, spelling) in [("64", "ng"), ("46", "hm"), ("464", "hng")] {
        let query = query(&mut engine, keys);
        assert!(
            query.pinyin_choices.iter().any(|c| c == spelling),
            "{keys}: {:?}",
            query.pinyin_choices
        );
    }
}

#[test]
fn pinned_ng_gives_en_first_and_commits_its_span() {
    // 64 选定 ng 后 嗯 排第一；后面还有数字时选了 嗯 剩下的接着打
    let mut engine = nine_key();
    engine.set_input("6452432");
    assert!(engine.pin_nine_key("ng"));
    assert_eq!(engine.composition().text(), "ng'52432");
    let query = engine.query().unwrap();
    assert_eq!(texts_of(&query)[0], "嗯");
    assert!(query.marked_text().starts_with("ng'"));
    let en = query.candidates.items[0].clone();
    engine.commit(&en);
    assert_eq!(engine.composition().text(), "52432");
}

#[test]
fn single_n_or_m_offers_the_interjection_but_stays_an_initial() {
    let mut engine = nine_key();
    engine.set_input("6");
    assert!(engine.pin_nine_key("m"));
    let query = engine.query().unwrap();
    assert!(texts_of(&query).iter().take(2).any(|t| *t == "呣"));
    // 后面还有键时 m 是简拼声母，不插 呣
    engine.set_input("6");
    assert!(engine.pin_nine_key("n"));
    engine.set_input("n'6");
    let query = engine.query().unwrap();
    assert!(!texts_of(&query).contains(&"嗯"));
}

#[test]
fn interjection_is_not_cut_out_of_a_longer_pinned_syllable() {
    // 选定 hang 后不该再切出 ha + ng
    let (shapes, _) = crate::nine_key::segment("hang");
    assert!(
        shapes
            .iter()
            .all(|shape| shape.positions.iter().all(|p| p.alternatives != ["ng"])),
        "{shapes:?}"
    );
}

#[test]
fn pinned_syllables_are_marked_and_back_out_whole() {
    let mut engine = nine_key();
    engine.set_input("52432");
    assert_eq!(engine.nine_key_pinned(), 0);
    assert!(engine.pin_nine_key("kai"));
    assert_eq!(engine.composition().text(), "kai'32");
    assert_eq!(engine.nine_key_pinned(), 3);
    assert!(engine.pin_nine_key("fa"));
    assert_eq!(engine.composition().text(), "kai'fa");
    assert_eq!(engine.nine_key_pinned(), 6);
    // 退格先把刚选定的 fa 整个退回数字，再一个个删数字
    engine.backspace();
    assert_eq!(engine.composition().text(), "kai'32");
    assert_eq!(engine.nine_key_pinned(), 3);
    engine.backspace();
    assert_eq!(engine.composition().text(), "kai'3");
    // 选定时接的 `'` 随最后一个键一起去掉
    engine.backspace();
    assert_eq!(engine.composition().text(), "kai");
    // 删到更早选定的 kai：一样先取消选定、退回数字，再删键
    engine.backspace();
    assert_eq!(engine.composition().text(), "524");
    assert_eq!(engine.nine_key_pinned(), 0);
}

#[test]
fn held_words_go_back_before_pinned_syllables() {
    let mut engine = nine_key();
    engine.set_input("52432");
    assert!(engine.pin_nine_key("kai"));
    let kai = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == "开")
        .unwrap();
    engine.commit(&kai);
    assert_eq!(engine.held_text(), "开");
    // 选定音节不把留着的词清掉
    assert!(engine.pin_nine_key("fa"));
    assert_eq!(engine.held_text(), "开");
    // 退格先撤回留着的词，再退选定的音节
    engine.backspace();
    assert!(engine.held_text().is_empty());
    assert_eq!(engine.composition().text(), "kai'fa");
    engine.backspace();
    assert_eq!(engine.composition().text(), "kai'32");
}

#[test]
fn an_unpinned_syllable_keeps_the_reading_the_user_picked() {
    // 64：词库里 米 比 你 常用，默认读 mi；选过 ni 再退回成数字仍读 ni
    let words = "米\tmi\t9000\n你\tni\t5000\n";
    let mut engine = Engine::new(Dictionary::parse(words).unwrap());
    engine.set_nine_key(true);
    engine.set_input("64");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "米");
    assert!(engine.pin_nine_key("ni"));
    engine.backspace();
    assert_eq!(engine.composition().text(), "64");
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "你");
    assert_eq!(query.marked_text(), "ni");
    // 再打新键就不再偏向
    engine.push('4');
    engine.backspace();
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "米");
}
