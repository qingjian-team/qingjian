//! 全局自动组词开关、词长与原有拼音阈值。

use super::{WordLearner, engine};
use crate::{Engine, Learner};

fn pair(engine: &mut Engine, first: &str, second: &str, same_buffer: bool) {
    let syllables = |text: &str| text.chars().map(|_| "kai".to_owned()).collect::<Vec<_>>();
    engine.record_word(first, &syllables(first), 2, true, same_buffer);
    engine.record_word(second, &syllables(second), 2, true, false);
    engine.note_passthrough('\n');
}

#[test]
fn auto_phrase_switch_and_length_preserve_pinyin_thresholds() {
    let mut engine = engine().with_learner(Box::<WordLearner>::default());
    engine.set_auto_phrase(false, 10);
    for _ in 0..3 {
        pair(&mut engine, "开发", "先", false);
    }
    assert!(engine.learner.user_words().is_none());
    assert!(
        engine
            .learner
            .user_ngram()
            .unwrap()
            .pair(Some("开发"), "先")
            > 0
    );

    engine.set_auto_phrase(true, 2);
    for _ in 0..3 {
        pair(&mut engine, "开发", "先", true);
    }
    assert!(engine.learner.user_words().is_none());

    engine.set_auto_phrase(true, 10);
    pair(&mut engine, "甲乙丙丁戊", "己庚辛壬癸", true);
    assert!(engine.learner.user_words().is_none());
    pair(&mut engine, "甲乙丙丁戊", "己庚辛壬癸", true);
    assert!(engine.learner.user_words().is_some());
    pair(&mut engine, "甲乙丙丁戊己", "庚辛壬癸子", true);
    pair(&mut engine, "甲乙丙丁戊己", "庚辛壬癸子", true);
    assert_eq!(engine.learner.user_words().unwrap().len(), 1);

    engine.set_auto_phrase(false, 4);
    assert_eq!(engine.learner.user_words().unwrap().len(), 1);
}

#[test]
fn auto_phrase_disabled_blocks_whole_buffer_learning() {
    let mut engine = engine().with_learner(Box::<WordLearner>::default());
    engine.set_auto_phrase(false, 10);
    for _ in 0..3 {
        engine.chain.begin_buffer("kaikai".into());
        engine.chain.advance("甲", &["kai".into()], true);
        engine.chain.advance("乙", &["kai".into()], false);
        engine.finish_buffer();
    }
    assert!(engine.learner.user_words().is_none());
}

#[test]
fn whole_buffer_length_can_reach_ten_but_not_eleven_characters() {
    for (first, second, expected) in [
        ("甲乙丙丁戊", "己庚辛壬癸", true),
        ("甲乙丙丁戊己", "庚辛壬癸子", false),
    ] {
        let mut engine = engine().with_learner(Box::<WordLearner>::default());
        engine.set_auto_phrase(true, 10);
        for round in 1..=2 {
            engine.chain.begin_buffer("kaikaikai".into());
            for text in [first, second] {
                let syllables = text.chars().map(|_| "kai".into()).collect::<Vec<_>>();
                engine.chain.advance(text, &syllables, false);
            }
            engine.finish_buffer();
            assert_eq!(
                engine.learner.user_words().is_some(),
                expected && round == 2
            );
        }
    }
}
