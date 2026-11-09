use super::{WordLearner, engine};
use crate::{Engine, Learner};

fn pair(engine: &mut Engine, first: &str, second: &str, same_buffer: bool) {
    let syllables = |text: &str| text.chars().map(|_| "kai".to_owned()).collect::<Vec<_>>();
    engine.record_word(first, &syllables(first), 2, true, same_buffer);
    engine.record_word(second, &syllables(second), 2, true, false);
    engine.note_passthrough('\n');
}

#[test]
fn wubi_options_preserve_pinyin_pair_thresholds_and_four_character_limit() {
    for (enabled, max_chars) in [(false, 2), (true, 2), (true, 4), (true, 10)] {
        let mut engine = engine().with_learner(Box::<WordLearner>::default());
        engine.set_auto_phrase(enabled, max_chars);
        pair(&mut engine, "开发", "先", true);
        assert_eq!(engine.learner.user_words().unwrap().len(), 1);
        for round in 1..=3 {
            pair(&mut engine, "甲乙", "丙丁", false);
            assert_eq!(
                engine.learner.user_words().unwrap().len(),
                1 + usize::from(round == 3)
            );
        }
        for _ in 0..3 {
            pair(&mut engine, "甲乙丙", "丁戊己", true);
        }
        assert_eq!(engine.learner.user_words().unwrap().len(), 2);
    }
}

#[test]
fn wubi_options_preserve_once_only_whole_buffer_learning_up_to_twenty_characters() {
    for (enabled, max_chars) in [(false, 2), (true, 2), (true, 4), (true, 10)] {
        for length in [4, 6, 10, 11, 20, 21] {
            let mut engine = engine().with_learner(Box::<WordLearner>::default());
            engine.set_auto_phrase(enabled, max_chars);
            engine.chain.begin_buffer("kai".repeat(length));
            for index in 0..length {
                engine
                    .chain
                    .advance("甲", &["kai".into()], index + 1 < length);
            }
            engine.finish_buffer(20);
            assert_eq!(
                engine.learner.user_words().is_some(),
                length <= 20,
                "enabled={enabled}, max_chars={max_chars}, length={length}"
            );
        }
    }
}

#[test]
fn changing_wubi_options_keeps_the_pinyin_commit_chain() {
    let mut engine = engine().with_learner(Box::<WordLearner>::default());
    engine.chain.begin_buffer("kaikai".into());
    engine.chain.advance("甲", &["kai".into()], true);
    engine.set_auto_phrase(false, 2);
    engine.chain.advance("乙", &["kai".into()], false);
    assert!(engine.finish_buffer(20).is_some());
    assert_eq!(engine.learner.user_words().unwrap().len(), 1);
}
