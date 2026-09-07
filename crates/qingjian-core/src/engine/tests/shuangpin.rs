//! 双拼。

use super::*;

#[test]
fn shuangpin_decodes_keys_before_lookup_and_shows_full_pinyin() {
    let mut engine = xiaohe();
    engine.set_input("kdfa");
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "开发");
    assert_eq!(query.marked_text(), "kai'fa");
    assert_eq!(query.marked_cursor(), 6);
    assert_eq!(query.text, "kdfa");
    assert!(query.shuangpin);
    // 末尾落单的键是声母前缀：`kdf` = kai f…
    engine.set_input("kdf");
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "kai'f");
    assert_eq!(query.candidates.items[0].text, "开放");
    // 配不出音节的键原样留作尾巴
    engine.set_input("kdbl");
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "kai'bl");
    assert_eq!(query.tail, "bl");
    assert_eq!(query.candidates.items[0].text, "开");
}

#[test]
fn shuangpin_commit_consumes_keys_per_syllable() {
    let mut engine = xiaohe();
    engine.set_input("kdfave");
    let query = engine.query().unwrap();
    let kaifa = query
        .candidates
        .items
        .iter()
        .find(|c| c.text == "开发")
        .cloned()
        .unwrap();
    engine.commit(&kaifa);
    assert_eq!(engine.composition().text(), "ve");
    assert_eq!(engine.query().unwrap().marked_text(), "zhe");
    // 未打完的最后一个键也被候选吃掉
    engine.set_input("kdf");
    let kaifang = engine.query().unwrap().candidates.items[0].clone();
    engine.commit(&kaifang);
    assert!(engine.composition().is_empty());
}

#[test]
fn shuangpin_records_choices_by_full_pinyin() {
    let mut engine = xiaohe();
    engine.set_input("kdfa");
    let query = engine.query().unwrap();
    // 两键定局：`fa` 不再按前缀放宽成 fan / fang
    assert!(query.candidates.items.iter().all(|c| c.text != "开饭"));
    let kaifa = query.candidates.items[0].clone();
    assert_eq!(kaifa.text, "开发");
    engine.commit(&kaifa);
    // 学习记的是全拼 kaifa：切回全拼、同样的拼音也受益
    assert!(engine.last_commit.as_ref().unwrap().same_input("kaifa"));
}

#[test]
fn shuangpin_gives_letter_mode_keys_back_to_syllables() {
    let mut engine = xiaohe();
    engine.set_input("v");
    assert!(!engine.expression_mode());
    assert_eq!(engine.query().unwrap().marked_text(), "zh");
    engine.set_input("u1");
    assert!(!engine.question_mode());
    // `?` 别名照常进问字
    engine.set_input("?nihc");
    assert!(engine.question_mode());
    assert_eq!(engine.query().unwrap().marked_text(), "?ni'hao");
}

#[test]
fn microsoft_semicolon_is_a_final_only_after_a_lone_initial() {
    let mut engine = engine();
    engine.set_shuangpin(Some(Scheme::Microsoft));
    engine.set_input("x");
    assert!(engine.takes_semicolon());
    engine.push(';');
    assert!(!engine.takes_semicolon());
    assert!(!engine.raw_mode());
    assert_eq!(engine.query().unwrap().marked_text(), "xing");
    engine.set_shuangpin(Some(Scheme::Xiaohe));
    engine.set_input("x");
    assert!(!engine.takes_semicolon());
}

#[test]
fn shuangpin_raw_commit_does_not_learn_decodable_keys_as_english() {
    let mut engine = xiaohe();
    engine.set_input("nihc");
    assert!(!looks_like_english_word_in(&engine));
    engine.set_input("gist");
    assert!(looks_like_english_word_in(&engine));
}
