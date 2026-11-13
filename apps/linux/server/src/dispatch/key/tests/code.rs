//! 纯形码与拼音混输的上屏、顶字和编辑规则。

use super::support::{compose, key, router};
use crate::dispatch::Router;
use qingjian_core::CodeScheme;
use qingjian_dictionary::CodeTable;
use qingjian_platform::protocol::{KeyModifiers, KeyOutcome};

pub(super) fn code_router(mixed: bool) -> Router {
    let mut router = router(5);
    let mut scheme = CodeScheme::new(
        "wubi86",
        CodeTable::parse(
            "一\tggll\t90000\n发\tv\t30000\n开\tga\t5000\n开发\tgant\t900\n甲\taaaa\t10\n乙\taaaa\t5\n泥\tni\t500\n",
        )
        .unwrap(),
    );
    scheme.auto_commit = true;
    scheme.top = true;
    router.engine.set_code_scheme(Some(scheme));
    router.engine.set_phonetic(mixed);
    router
}

#[test]
fn unique_full_code_commits_without_space() {
    let mut router = code_router(false);
    let modifiers = KeyModifiers::default();
    compose(&mut router, "ggl", modifiers);
    let (outcome, commit, frame) = key(&mut router, b'l' as u32, Some('l'), modifiers);
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert_eq!(commit.as_deref(), Some("一"));
    assert!(frame.candidates.items.is_empty());
    assert!(router.engine.composition().is_empty());
}

#[test]
fn topping_commits_highlight_and_keeps_next_code() {
    let mut router = code_router(false);
    let modifiers = KeyModifiers::default();
    compose(&mut router, "aaaa", modifiers);
    key(&mut router, 0x28, None, modifiers);
    let (_, commit, frame) = key(&mut router, b'g' as u32, Some('g'), modifiers);
    assert_eq!(commit.as_deref(), Some("乙"));
    assert_eq!(router.engine.composition().text(), "g");
    assert_eq!(frame.candidates.items[0].text, "一");
    assert_eq!(
        key(&mut router, 32, Some(' '), modifiers).1.as_deref(),
        Some("一")
    );
}

#[test]
fn ambiguous_code_waits_for_selection() {
    let mut router = code_router(false);
    let modifiers = KeyModifiers::default();
    compose(&mut router, "aaa", modifiers);
    let (_, commit, frame) = key(&mut router, b'a' as u32, Some('a'), modifiers);
    assert!(commit.is_none());
    assert_eq!(frame.candidates.items.len(), 2);
    assert_eq!(
        key(&mut router, b'2' as u32, Some('2'), modifiers)
            .1
            .as_deref(),
        Some("乙")
    );
}

#[test]
fn disabled_rules_do_not_commit_automatically() {
    let mut router = code_router(false);
    router.engine.set_code_rules(false, false);
    let modifiers = KeyModifiers::default();
    compose(&mut router, "ggll", modifiers);
    assert_eq!(router.engine.composition().text(), "ggll");
    assert!(
        key(&mut router, b'g' as u32, Some('g'), modifiers)
            .1
            .is_none()
    );
    assert_eq!(router.engine.composition().text(), "ggllg");
}

#[test]
fn mixed_input_keeps_pinyin_and_does_not_auto_commit() {
    let mut router = code_router(true);
    let modifiers = KeyModifiers::default();
    compose(&mut router, "ni", modifiers);
    let texts = router
        .current_frame()
        .candidates
        .items
        .into_iter()
        .map(|candidate| candidate.text)
        .collect::<Vec<_>>();
    assert!(texts.iter().any(|text| text == "你"));
    assert!(texts.iter().any(|text| text == "泥"));
    key(&mut router, 27, None, modifiers);
    compose(&mut router, "ggll", modifiers);
    assert_eq!(router.engine.composition().text(), "ggll");
    assert!(
        key(&mut router, b'g' as u32, Some('g'), modifiers)
            .1
            .is_none()
    );
}

#[test]
fn code_letters_and_editing_do_not_enter_pinyin_modes() {
    let mut router = code_router(false);
    let modifiers = KeyModifiers::default();
    compose(&mut router, "v", modifiers);
    assert!(!router.engine.expression_mode());
    assert_eq!(
        key(&mut router, 32, Some(' '), modifiers).1.as_deref(),
        Some("发")
    );
    compose(&mut router, "gan", modifiers);
    key(&mut router, 8, None, modifiers);
    assert_eq!(router.engine.composition().text(), "ga");
    assert_eq!(
        key(&mut router, b',' as u32, Some(','), modifiers)
            .1
            .as_deref(),
        Some("开，")
    );
    compose(&mut router, "zzzzg", modifiers);
    assert_eq!(
        key(&mut router, 13, None, modifiers).1.as_deref(),
        Some("zzzzg")
    );
}

#[test]
fn uppercase_compose_preserves_existing_core_rules() {
    let mut router = code_router(false);
    router.config.shift_letter_compose = true;
    let modifiers = KeyModifiers {
        shift: true,
        ..Default::default()
    };
    compose(&mut router, "GGL", modifiers);
    assert!(
        key(&mut router, b'L' as u32, Some('L'), modifiers)
            .1
            .is_none()
    );
    assert_eq!(
        key(&mut router, 32, Some(' '), KeyModifiers::default())
            .1
            .as_deref(),
        Some("GGLL ")
    );
}

#[test]
fn empty_code_is_discarded_on_space_and_punctuation() {
    let mut router = code_router(false);
    let modifiers = KeyModifiers::default();
    compose(&mut router, "zzzz", modifiers);
    assert!(key(&mut router, 32, Some(' '), modifiers).1.is_none());
    assert!(router.engine.composition().is_empty());
    compose(&mut router, "zzzz", modifiers);
    assert_eq!(
        key(&mut router, b',' as u32, Some(','), modifiers)
            .1
            .as_deref(),
        Some("，")
    );
}
