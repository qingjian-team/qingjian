//! 自造词与简拼学习：同一段拼音逐字选一次就成词、全拼选过的词简拼也排前，下次整段与简拼都排第一。

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

#[test]
fn a_word_picked_by_full_pinyin_leads_its_abbreviation() {
    // 词库里的词：用全拼选过一次，下次打简拼也排第一（选择按简拼键也记一份）
    let dictionary = "事实\tshi shi\t9000\n实施\tshi shi\t5000\n是\tshi\t9000\n";
    let mut engine = Engine::new(Dictionary::parse(dictionary).unwrap())
        .with_learner(Box::new(WordLearner::default()));
    engine.set_input("ss");
    assert_eq!(texts_of(&engine)[0], "事实");
    engine.clear();
    engine.set_input("shishi");
    select(&mut engine, "实施");
    engine.note_passthrough('\n');
    engine.set_input("ss");
    assert_eq!(texts_of(&engine)[0], "实施");
}

#[test]
fn the_same_word_typed_twice_does_not_make_a_doubled_word() {
    let mut engine = shishan_engine();
    engine.set_input("shishan");
    select(&mut engine, "屎");
    select(&mut engine, "山");
    for _ in 0..4 {
        engine.set_input("shishan");
        select(&mut engine, "屎山");
    }
    let patterns: Vec<_> = ["shi", "shan", "shi", "shan"]
        .map(qingjian_dictionary::SyllablePattern::complete)
        .to_vec();
    let doubled = engine.learner().user_words().is_some_and(|words| {
        words
            .lookup_exact(&patterns)
            .iter()
            .any(|hit| hit.text == "屎山屎山")
    });
    assert!(!doubled);
}

#[test]
fn a_chosen_low_tier_word_survives_thousands_of_abbreviation_hits() {
    // 补充词库里的词（低主词库一档）在 `ss` 下选过：几千个主词库两字词预选时也不把它砍掉，按选择排第一
    let mut base = String::from("是\tshi\t9000\n");
    for n in 0..3000 {
        base.push_str(&format!("森森{n}\tsen sen\t{}\n", 9000 - n % 100));
    }
    let mut general = Dictionary::parse("屎山\tshi shan\t10\n").unwrap();
    general.set_secondary(true);
    let mut engine = Engine::new(Dictionary::parse(&base).unwrap())
        .with_learner(Box::new(WordLearner::default()));
    engine.set_extra_dictionaries(vec![general]);
    engine.set_input("shishan");
    select(&mut engine, "屎山");
    engine.note_passthrough('\n');
    engine.set_input("ss");
    assert_eq!(texts_of(&engine)[0], "屎山");
}

/// 词库有 交易、中心，没有 交易中心；简拼 jyzx 下 就要、主席 词频更高。
fn jiaoyi_engine() -> Engine {
    let dictionary = concat!(
        "交易\tjiao yi\t5000\n中心\tzhong xin\t5000\n就要\tjiu yao\t20000\n主席\tzhu xi\t20000\n",
        "区\tqu\t9000\n",
    );
    Engine::new(Dictionary::parse(dictionary).unwrap())
        .with_learner(Box::new(WordLearner::default()))
}

fn commit_sentence(engine: &mut Engine, input: &str) -> String {
    engine.set_input(input);
    let first = engine.query().unwrap().candidates.items.remove(0);
    assert_eq!(first.kind, CandidateKind::Sentence, "{input}");
    engine.commit(&first)
}

#[test]
fn a_short_sentence_picked_once_becomes_a_word_for_its_abbreviation() {
    // #875：全拼一次选中的陌生词（整句 交易中心），下次简拼 jyzx 也排第一
    let mut engine = jiaoyi_engine();
    engine.set_input("jyzx");
    assert_ne!(texts_of(&engine)[0], "交易中心");
    engine.clear();
    assert_eq!(commit_sentence(&mut engine, "jiaoyizhongxin"), "交易中心");
    engine.note_passthrough('\n');
    for input in ["jiaoyizhongxin", "jyzx"] {
        engine.set_input(input);
        assert_eq!(texts_of(&engine)[0], "交易中心", "{input}");
        engine.clear();
    }
}

#[test]
fn a_sentence_picked_once_with_a_single_char_or_over_six_chars_is_not_a_word() {
    // 一次接受的整句带单字（交易 + 区）多半是句子，超过 6 个字的多半是模型首选，都不记成用户词
    let mut engine = jiaoyi_engine();
    assert_eq!(commit_sentence(&mut engine, "jiaoyiqu"), "交易区");
    assert_eq!(engine.learner().weight("交易区"), 0);
    engine.note_passthrough('\n');
    assert_eq!(
        commit_sentence(&mut engine, "jiaoyizhongxinjiaoyizhongxin"),
        "交易中心交易中心"
    );
    assert_eq!(engine.learner().weight("交易中心交易中心"), 0);
    engine.note_passthrough('\n');
    assert_eq!(
        commit_sentence(&mut engine, "jiaoyizhongxinzhongxin"),
        "交易中心中心"
    );
    assert_eq!(engine.learner().weight("交易中心中心"), 1);
}

#[test]
fn erasing_the_sentence_and_picking_another_forgets_the_new_word() {
    // 按空格上屏了 交易中心，删掉重打改选 交易：刚造的词一并删掉
    let mut engine = jiaoyi_engine();
    commit_sentence(&mut engine, "jiaoyizhongxin");
    assert_eq!(engine.learner().weight("交易中心"), 1);
    for _ in 0..4 {
        engine.note_backspace();
    }
    engine.set_input("jiaoyizhongxin");
    select(&mut engine, "交易");
    assert_eq!(engine.learner().weight("交易中心"), 0);
}
