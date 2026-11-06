//! 自造词：同一段拼音逐字选一次就成词，下次整段与简拼都排第一。

use super::*;

use crate::ranking::{USER_WORDS_BEFORE_SINGLES, WORDS_BEFORE_SINGLE_CHARS};

/// 词库里有 石山、事实 和 是 屎 山 几个单字，没有 屎山；整句会读成 是山。
fn shishan_engine() -> Engine {
    let dictionary =
        "石山\tshi shan\t900\n事实\tshi shi\t9000\n是\tshi\t9000\n屎\tshi\t10\n山\tshan\t9000\n";
    Engine::new(Dictionary::parse(dictionary).unwrap())
        .with_learner(Box::new(WordLearner::default()))
}

fn select(engine: &mut Engine, text: &str) {
    let candidate = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == text)
        .unwrap_or_else(|| panic!("没有 {text}"));
    engine.commit(&candidate);
}

#[test]
fn picking_the_chars_once_makes_a_word_that_leads_the_whole_input_and_its_abbreviation() {
    let mut engine = shishan_engine();
    engine.set_input("shishan");
    select(&mut engine, "屎");
    select(&mut engine, "山");
    assert!(engine.composition().is_empty());
    engine.note_passthrough('\n');
    for input in ["shishan", "ss", "shis"] {
        engine.set_input(input);
        assert_eq!(texts_of(&engine)[0], "屎山", "{input}");
        engine.clear();
    }
}

#[test]
fn only_a_few_user_words_skip_the_word_quota_before_single_chars() {
    // 简拼 `ss` 对得上十二个自造词：至多 USER_WORDS_BEFORE_SINGLES 个不占名额，其余照常占
    // WORDS_BEFORE_SINGLE_CHARS 个名额，首音节的单字不被挤到后面
    let mut learner = WordLearner::default();
    for c in "一二三四五六七八九十百千".chars() {
        learner.learn_word(&format!("书{c}"), &["shu".to_owned(), "san".to_owned()]);
    }
    let dictionary = "是\tshi\t9000\n书\tshu\t9000\n山\tshan\t9000\n";
    let mut engine =
        Engine::new(Dictionary::parse(dictionary).unwrap()).with_learner(Box::new(learner));
    engine.set_input("ss");
    let all = texts_of(&engine);
    let first_char = all
        .iter()
        .position(|t| t.chars().count() == 1)
        .unwrap_or_else(|| panic!("{all:?}"));
    assert!(
        first_char <= USER_WORDS_BEFORE_SINGLES + WORDS_BEFORE_SINGLE_CHARS,
        "{all:?}"
    );
}

#[test]
fn a_sentence_picked_in_pieces_comes_first_for_its_pinyin_and_abbreviation() {
    // 整句记忆：逐字纠正过的整句（人名不在词库里）下次全拼与整句简拼都排第一
    let dictionary = concat!(
        "小\txiao\t9000\n晓\txiao\t10\n明\tming\t9000\n铭\tming\t10\n小明\txiao ming\t9000\n",
        "去了\tqu le\t9000\n学校\txue xiao\t9000\n",
    );
    let mut engine = Engine::new(Dictionary::parse(dictionary).unwrap())
        .with_learner(Box::new(WordLearner::default()));
    engine.set_input("xiaomingqulexuexiao");
    for text in ["晓", "铭", "去了", "学校"] {
        select(&mut engine, text);
    }
    assert!(engine.composition().is_empty());
    engine.note_passthrough('\n');
    for input in ["xiaomingqulexuexiao", "xmqlxx"] {
        engine.set_input(input);
        assert_eq!(texts_of(&engine)[0], "晓铭去了学校", "{input}");
        engine.clear();
    }
}
