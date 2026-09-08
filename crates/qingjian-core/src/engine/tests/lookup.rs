//! 查词、切分、光标与上屏消耗。

use super::*;

#[test]
fn exact_word_first_then_longer_then_prefix_expansions_then_prefix_words() {
    assert_eq!(texts("kaifa"), ["开发", "开放", "开饭", "开发者", "开"]);
}

#[test]
fn partial_last_syllable_expands() {
    assert_eq!(texts("kaif"), ["开放", "开发", "开饭", "开发者", "开"]);
}

#[test]
fn initials_match_abbreviated_words_and_commit_consumes_letters() {
    let all = texts("kf");
    assert_eq!(all[0], "开放"); // 同为简拼命中，按词频
    assert!(all.contains(&"开发".to_owned()) && all.contains(&"咖啡".to_owned()));

    let mut engine = engine();
    engine.set_input("kfzhe");
    let kaifa = engine
        .query()
        .unwrap()
        .candidates
        .items
        .iter()
        .find(|c| c.text == "开发")
        .unwrap()
        .clone();
    engine.commit(&kaifa);
    assert_eq!(engine.composition().text(), "zhe");
}

#[test]
fn unparsable_tail_is_kept_aside() {
    let mut engine = engine();
    // kaiv 唯一的纠法是删掉刚敲的 v，那不算纠错：v 留作尾巴等下一键（kaifv 会被纠成 kaifa）
    engine.set_input("kaiv");
    let query = engine.query().unwrap();
    assert!(query.correction.is_none());
    assert_eq!(query.tail, "v");
    assert_eq!(query.marked_text(), "kai'v");
    assert_eq!(query.candidates.items[0].text, "开");

    let kaifa = query
        .candidates
        .items
        .iter()
        .find(|c| c.text == "开发")
        .unwrap()
        .clone();
    engine.commit(&kaifa);
    assert_eq!(engine.composition().text(), "v");
    // 剩下的 v 进表达式模式：没候选但也不报错
    assert!(engine.query().unwrap().candidates.items.is_empty());
}

#[test]
fn cursor_edits_requery_from_the_start_and_map_into_marked_text() {
    let mut engine = engine();
    engine.set_input("kaifa");
    engine.move_cursor_left();
    engine.move_cursor_left();
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "kai'fa");
    assert_eq!(query.marked_cursor(), 3); // kai|'fa

    engine.push('n');
    let query = engine.query().unwrap();
    assert_eq!(query.text, "kainfa");
    assert_eq!(query.marked_cursor(), 5); // kai'n|'fa

    engine.set_input("xi'an");
    engine.move_cursor_left();
    engine.move_cursor_left();
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "xi'an");
    assert_eq!(query.marked_cursor(), 3); // xi'|an，紧跟用户自己敲的 '
}

#[test]
fn punctuation_follows_committed_text() {
    let mut engine = engine();
    assert_eq!(engine.punctuate(','), Some("，"));
    engine.note_passthrough('3');
    assert_eq!(engine.punctuate('.'), None);
    engine.set_input("kaifa");
    let kaifa = engine.query().unwrap().candidates.items[0].clone();
    engine.commit(&kaifa);
    assert_eq!(engine.punctuate('.'), Some("。"));
}

#[test]
fn marked_text_joins_best_segmentation_with_apostrophes() {
    let mut engine = engine();
    engine.set_input("kaifa");
    assert_eq!(engine.query().unwrap().marked_text(), "kai'fa");
    engine.set_input("kf");
    assert_eq!(engine.query().unwrap().marked_text(), "k'f");
}

#[test]
fn prefix_words_appear_after_full_matches() {
    // 开放 / 开饭 来自 `kai f… a zhe` 这种切分的前缀 `kai f…`，排在覆盖更多字母的 开发 之后
    let all = texts("kaifazhe");
    assert_eq!(&all[..2], ["开发者", "开发"]);
    assert_eq!(all.last().map(String::as_str), Some("开"));
}

#[test]
fn ambiguous_segmentation_merges_results() {
    let all = texts("xian");
    assert_eq!(all[0], "先");
    assert!(all.contains(&"西安".to_owned()));
}

#[test]
fn complete_syllable_that_is_also_prefix_expands_after_exact() {
    // 下 是完整命中；先 / 想 来自 xia → xian / xiang 的前缀扩展；西安 来自 xi + a… 的次级切分
    assert_eq!(texts("xia"), ["下", "先", "想", "西安"]);
}

#[test]
fn empty_input_is_an_error() {
    assert_eq!(engine().query().unwrap_err(), ParseError::Empty);
}

#[test]
fn commit_consumes_only_the_candidate_syllables() {
    let mut engine = engine();
    engine.set_input("kaifazhe");
    let kaifa = engine
        .query()
        .unwrap()
        .candidates
        .items
        .iter()
        .find(|c| c.text == "开发")
        .unwrap()
        .clone();
    assert_eq!(engine.commit(&kaifa), "开发");
    assert_eq!(engine.composition().text(), "zhe");

    engine.set_input("kaif");
    assert_eq!(engine.commit(&kaifa), "开发");
    assert!(engine.composition().is_empty());

    engine.set_input("xi'an");
    let xian = engine
        .query()
        .unwrap()
        .candidates
        .items
        .iter()
        .find(|c| c.text == "西安")
        .unwrap()
        .clone();
    engine.commit(&xian);
    assert!(engine.composition().is_empty());
}

#[test]
fn take_raw_returns_pinyin_and_clears() {
    let mut engine = engine();
    engine.set_input("kaifa");
    assert_eq!(engine.take_raw(), "kaifa");
    assert!(engine.composition().is_empty());
}

#[test]
fn choice_key_strips_separators_and_clamps() {
    assert_eq!(choice_key("kai'fa", 6), "kaifa");
    assert_eq!(choice_key("kai'fa", 3), "kai");
    assert_eq!(choice_key("ba", 10), "ba");
}

#[test]
fn sentence_conversion_leads_when_input_spans_several_words() {
    // 想开发：SAMPLE 里没有整词，最优路径是 想 + 开发
    let all = texts("xiangkaifa");
    assert_eq!(all[0], "想开发");
    let mut engine = engine();
    engine.set_input("xiangkaifa");
    let query = engine.query().unwrap();
    let sentence = query.candidates.items[0].clone();
    assert_eq!(sentence.kind, CandidateKind::Sentence);
    assert_eq!(sentence.syllables, ["xiang", "kai", "fa"]);
    assert_eq!(engine.commit(&sentence), "想开发");
    assert!(engine.composition().is_empty());

    // 整段本身是一个词：不出整句
    let all = texts("kaifa");
    assert_eq!(all[0], "开发");
    assert_eq!(all.iter().filter(|t| *t == "开发").count(), 1);

    // 末尾只有一个字母时整句不算它：xiangkaif → 想开
    let all = texts("xiangkaif");
    assert_eq!(all[0], "想开");
}

#[test]
fn shortcuts_follow_the_first_local_candidate() {
    let all = texts("rq");
    let shortcut = all.iter().position(|t| t.ends_with('日')).unwrap();
    assert!(shortcut <= 1);
    assert!(all.iter().any(|t| t.contains('-')));

    let mut engine = engine();
    engine.set_input("xq");
    let query = engine.query().unwrap();
    let weekday = query
        .candidates
        .items
        .iter()
        .find(|c| c.kind == CandidateKind::Shortcut)
        .unwrap()
        .clone();
    assert!(weekday.text.starts_with("星期"));
    engine.commit(&weekday);
    assert!(engine.composition().is_empty());
}

#[test]
fn expression_mode_skips_pinyin_and_evaluates() {
    let mut engine = self::engine();
    assert!(!engine.expression_mode());
    engine.set_input("v1+2");
    assert!(engine.expression_mode());
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "v1+2");
    assert_eq!(query.marked_cursor(), 4);
    assert_eq!(query.candidates.items[0].text, "3");
    assert_eq!(query.candidates.items[0].kind, CandidateKind::Shortcut);
    assert_eq!(query.candidates.items[1].text, "1+2=3");
    let result = query.candidates.items[0].clone();
    assert_eq!(engine.commit(&result), "3");
    assert!(engine.composition().is_empty());

    // 只有 v：候选为空但不报错，preedit 照显示
    engine.set_input("v");
    let query = engine.query().unwrap();
    assert!(query.candidates.items.is_empty());
    assert_eq!(query.marked_text(), "v");

    // v 开头的英文词仍能混输
    let words = WordList::parse("very\n").unwrap();
    let mut engine = self::engine().with_english(words);
    engine.set_input("very");
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "very");
    assert_eq!(query.candidates.items[0].kind, CandidateKind::English);
}

/// 只认句首的 开发 与 开发 → 先 的假模型：让两词路径压过整段的词。
struct XianModel;

impl LanguageModel for XianModel {
    fn log_prob(&self, previous: Option<&str>, word: &str) -> Option<f64> {
        match (previous, word) {
            (None, "开发") => Some(-1.0),
            (Some("开发"), "先") => Some(-0.5),
            _ => None,
        }
    }
}

#[test]
fn a_word_spelling_the_sentence_keeps_its_rank_unless_its_reading_differs() {
    // 词级排序里 开发线 在 开发先 前面；整句转换读出的是 开发 + 先，与词 开发先 同文本同读音：不重复插，词留在原位
    let sample = format!("{SAMPLE}开发线\tkai fa xian\t5000\n开发先\tkai fa xian\t1\n");
    let mut engine =
        Engine::new(Dictionary::parse(&sample).unwrap()).with_language_model(Box::new(XianModel));
    engine.set_input("kaifaxian");
    let all = texts_of(&engine);
    assert_eq!(&all[..2], ["开发线", "开发先"]);
    assert_eq!(all.iter().filter(|t| *t == "开发先").count(), 1);

    // 同文本的词是按别的读音（xiang，靠模糊音 an-ang 对上）收的：那条错读音的词让位，整句以正确读音排最前
    let sample = format!("{SAMPLE}开发线\tkai fa xian\t5000\n开发先\tkai fa xiang\t1\n");
    let mut engine =
        Engine::new(Dictionary::parse(&sample).unwrap()).with_language_model(Box::new(XianModel));
    engine.set_fuzzy(FuzzyRules {
        an_ang: true,
        ..FuzzyRules::default()
    });
    engine.set_input("kaifaxian");
    let items = engine.query().unwrap().candidates.items;
    assert_eq!(items[0].text, "开发先");
    assert_eq!(items[0].kind, CandidateKind::Sentence);
    assert_eq!(items[0].syllables, ["kai", "fa", "xian"]);
    assert_eq!(items.iter().filter(|c| c.text == "开发先").count(), 1);
}
