//! 屏蔽词：候选里含有屏蔽词的整条都不出。

use super::*;

fn engine_with(words: &str) -> Engine {
    Engine::new(Dictionary::parse(words).unwrap())
}

#[test]
fn blocked_words_hide_the_candidate_that_contains_them() {
    let mut engine = engine_with("搜狗\tsou gou\t9000\n搜\t sou\t50\n开发\tkai fa\t9000\n");
    engine.set_input("sougou");
    assert!(texts_of(&engine).iter().any(|text| text == "搜狗"));

    engine.set_blocked_words(vec!["搜狗".to_owned()]);
    engine.set_input("sougou");
    assert!(!texts_of(&engine).iter().any(|text| text.contains("搜狗")));
    // 别的词不受影响
    engine.set_input("kaifa");
    assert_eq!(texts_of(&engine).first().map(String::as_str), Some("开发"));
}

/// 屏蔽词按「含有」算：挡掉 `搜狗` 也挡掉 `搜狗拼音`、`搜狗输入法`。
#[test]
fn blocked_words_also_hide_longer_words_containing_them() {
    let mut engine = engine_with(
        "搜狗\tsou gou\t9000\n搜狗拼音\tsou gou pin yin\t8000\n搜狗输入法\tsou gou shu ru fa\t7000\n",
    );
    engine.set_blocked_words(vec!["搜狗".to_owned()]);
    engine.set_input("sougou");
    let texts = texts_of(&engine);
    assert!(!texts.iter().any(|text| text.contains("搜狗")));
}

/// 自定义短语是用户自己写下的原文，屏蔽词不管它。
#[test]
fn blocked_words_leave_custom_phrases_alone() {
    let phrase = crate::CustomPhrase {
        code: "sg".to_owned(),
        text: "搜狗".to_owned(),
        weight: 1,
        enabled: true,
    };
    let mut engine = engine_with("搜狗\tsou gou\t9000\n");
    engine.set_custom_phrases(vec![phrase]).unwrap();
    engine.set_blocked_words(vec!["搜狗".to_owned()]);
    engine.set_input("sg");
    assert!(texts_of(&engine).iter().any(|text| text == "搜狗"));
}

/// 空串不参与匹配：`""` 会被 `contains` 当成命中一切。
#[test]
fn empty_blocked_word_is_ignored() {
    let mut engine = engine_with("开发\tkai fa\t9000\n");
    engine.set_blocked_words(vec![String::new()]);
    engine.set_input("kaifa");
    assert_eq!(texts_of(&engine).first().map(String::as_str), Some("开发"));
}
