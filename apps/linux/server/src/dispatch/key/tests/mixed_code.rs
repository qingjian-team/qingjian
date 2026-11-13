//! 五笔混输的四码自动上屏与第五键续输，经过 Linux 按键路由验证。

use qingjian_core::CandidateKind;
use qingjian_platform::protocol::KeyModifiers;

use super::code::code_router;
use super::support::{compose, key};

#[test]
fn mixed_unique_code_commits_only_on_the_fourth_key() {
    let mut router = code_router(true);
    router.engine.set_mixed_code_rules(true, false);
    let modifiers = KeyModifiers::default();
    for character in "ggl".chars() {
        assert!(
            key(&mut router, character as u32, Some(character), modifiers)
                .1
                .is_none()
        );
    }
    let (_, commit, frame) = key(&mut router, b'l' as u32, Some('l'), modifiers);
    assert_eq!(commit.as_deref(), Some("一"));
    assert!(frame.candidates.items.is_empty());
    assert!(router.engine.composition().is_empty());
}

#[test]
fn mixed_ambiguous_code_tops_the_highlight_and_keeps_the_fifth_key() {
    let mut router = code_router(true);
    router.engine.set_mixed_code_rules(true, true);
    let modifiers = KeyModifiers::default();
    compose(&mut router, "aaaa", modifiers);
    assert_eq!(router.engine.composition().text(), "aaaa");
    key(&mut router, 0x28, None, modifiers);
    let (_, commit, frame) = key(&mut router, b'g' as u32, Some('g'), modifiers);
    assert_eq!(commit.as_deref(), Some("乙"));
    assert_eq!(router.engine.composition().text(), "g");
    assert_eq!(frame.candidates.items[0].kind, CandidateKind::Code);
    assert_eq!(frame.candidates.items[0].text, "一");
    compose(&mut router, "gl", modifiers);
    assert_eq!(
        key(&mut router, b'l' as u32, Some('l'), modifiers)
            .1
            .as_deref(),
        Some("一")
    );
    assert!(router.engine.composition().is_empty());
}

#[test]
fn mixed_topping_can_be_enabled_without_auto_commit() {
    let mut router = code_router(true);
    router.engine.set_mixed_code_rules(false, true);
    let modifiers = KeyModifiers::default();
    compose(&mut router, "ggll", modifiers);
    assert_eq!(router.engine.composition().text(), "ggll");
    assert_eq!(
        key(&mut router, b'g' as u32, Some('g'), modifiers)
            .1
            .as_deref(),
        Some("一")
    );
    assert_eq!(router.engine.composition().text(), "g");
}

#[test]
fn mixed_auto_commit_does_not_enable_topping() {
    let mut router = code_router(true);
    router.engine.set_mixed_code_rules(true, false);
    let modifiers = KeyModifiers::default();
    compose(&mut router, "aaaa", modifiers);
    assert!(
        key(&mut router, b'g' as u32, Some('g'), modifiers)
            .1
            .is_none()
    );
    assert_eq!(router.engine.composition().text(), "aaaag");
}
