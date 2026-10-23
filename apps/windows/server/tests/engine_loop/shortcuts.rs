//! 组句中「修饰键 + 数字」：译词上屏。

use crate::composing::SentenceTranslator;
use crate::support::*;

#[test]
fn alt_digit_commits_first_translation() {
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let slot = slot_of(&frame, "你好");
    // 缺省译词键（mac ⌥ / Windows Ctrl）+ 数字：上屏那个候选的第一个译词，组句结束。
    let (outcome, commit, after) = press(&mut router, digit_with(slot, TRANSLATE));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("hello"))
    );
    assert!(after.is_empty());
}

#[test]
fn alt_digit_commits_a_sentence_translation() {
    let submitted = Arc::new(Mutex::new(Vec::new()));
    let mut router = router_with(RouterConfig {
        sentence_translation: true,
        ..RouterConfig::default()
    });
    router
        .engine_mut()
        .set_predictor(Box::new(SentenceTranslator::new(submitted)));
    let (_, _, frame) = type_letters(&mut router, "xiangkaifa");
    let slot = slot_of(&frame, "想开发");
    let (outcome, commit, after) = press(&mut router, digit_with(slot, TRANSLATE));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("It seems not to work."))
    );
    assert!(after.is_empty());
}

#[test]
fn second_translation_key_without_second_sense_is_swallowed() {
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let slot = slot_of(&frame, "你好");
    // 样例释义表里「你好」只有一条译文：第二个译词键（Shift+译词键）+ 数字吞掉不动，组句还在。
    let (outcome, commit, after) = press(&mut router, digit_with(slot, TRANSLATE_SECOND));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&after), "ni'hao");
}

#[test]
fn unconfigured_modifier_digit_is_not_a_selection() {
    // Shift+4 就是普通的 `$`；Win+1 没配到快捷键，归应用。
    let mut router = router_with(RouterConfig {
        // 组句中标点进直输段的老行为（`punct_commits` 关），`$` 才会留在拼音行里
        punct_commits: false,
        ..RouterConfig::default()
    });
    type_letters(&mut router, "nihao");
    let (outcome, commit, frame) = press(&mut router, digit_with(4, SHIFT));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert!(preedit(&frame).contains('$'), "{}", preedit(&frame));
    let (outcome, commit, _) = press(&mut router, digit_with(1, WIN));
    assert_eq!((outcome, commit), (KeyOutcome::Passthrough, None));
}
