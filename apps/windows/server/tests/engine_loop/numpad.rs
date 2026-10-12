//! 小键盘数字配置：选词、直输保序与其他模式的边界。

use qingjian_platform::NumpadDigit;

use crate::support::{
    CAPS, ENGLISH, KeyEvent, KeyModifiers, KeyOutcome, Router, RouterConfig, TRANSLATE, WIN, digit,
    digit_with, preedit, press, router_asking_with, router_with, slot_of, type_english,
    type_letters,
};

fn keypad(n: u32, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(0x60 + n, char::from_digit(n, 10), modifiers)
}

fn direct_router() -> Router {
    router_with(RouterConfig {
        numpad_digit: NumpadDigit::Direct,
        ..RouterConfig::default()
    })
}

#[test]
fn default_keypad_selects_current_page_candidate() {
    let mut router = router_with(RouterConfig::default());
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let n = slot_of(&frame, "你好");
    let (outcome, commit, frame) = press(&mut router, keypad(n, KeyModifiers::default()));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert_eq!(commit.as_deref(), Some("你好"));
    assert!(frame.is_empty());
}

#[test]
fn direct_keypad_digits_keep_input_order_until_space_or_enter() {
    for text in ["nihao", "wenti", "han"] {
        for n in 0..=9 {
            for (code, character, suffix) in [(0x20, Some(' '), " "), (0x0D, None, "")] {
                let mut router = direct_router();
                type_letters(&mut router, text);
                let (outcome, commit, frame) =
                    press(&mut router, keypad(n, KeyModifiers::default()));
                assert_eq!(outcome, KeyOutcome::Consumed);
                assert_eq!(commit, None);
                assert_eq!(preedit(&frame), format!("{text}{n}"));
                type_letters(&mut router, "gpt");
                let (_, commit, frame) = press(
                    &mut router,
                    KeyEvent::new(code, character, KeyModifiers::default()),
                );
                assert_eq!(
                    commit.as_deref(),
                    Some(format!("{text}{n}gpt{suffix}").as_str())
                );
                assert!(frame.is_empty());
            }
        }
    }
}

#[test]
fn direct_does_not_change_main_digits_or_translation_shortcuts() {
    let mut router = direct_router();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let n = slot_of(&frame, "你好");
    assert_eq!(press(&mut router, digit(n)).1.as_deref(), Some("你好"));
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let n = slot_of(&frame, "你好");
    assert_eq!(
        press(&mut router, digit_with(n, TRANSLATE)).1.as_deref(),
        Some("hello")
    );
}

#[test]
fn digit_without_a_candidate_slot_still_enters_buffer_in_select_mode() {
    let mut router = router_with(RouterConfig {
        page_size: 1,
        ..RouterConfig::default()
    });
    type_letters(&mut router, "nihao");
    let (_, commit, frame) = press(&mut router, keypad(9, KeyModifiers::default()));
    assert_eq!(commit, None);
    assert_eq!(preedit(&frame), "nihao9");
}

#[test]
fn english_caps_expression_and_command_keys_keep_their_semantics() {
    let mut router = direct_router();
    let (_, _, frame) = type_english(&mut router, "hello");
    let n = slot_of(&frame, "hello");
    assert_eq!(
        press(&mut router, keypad(n, ENGLISH)).1.as_deref(),
        Some("hello")
    );
    assert_eq!(
        press(&mut router, keypad(5, CAPS)).0,
        KeyOutcome::Passthrough
    );
    type_letters(&mut router, "v");
    press(&mut router, keypad(1, KeyModifiers::default()));
    press(
        &mut router,
        KeyEvent::new(0x6B, Some('+'), KeyModifiers::default()),
    );
    let (_, _, frame) = press(&mut router, keypad(2, KeyModifiers::default()));
    assert!(frame.candidates.items.iter().any(|c| c.text == "3"));
    let before = preedit(&frame);
    let (outcome, commit, after) = press(&mut router, keypad(5, WIN));
    assert_eq!(outcome, KeyOutcome::Passthrough);
    assert_eq!(commit, None);
    assert_eq!(preedit(&after), before);
}

#[test]
fn keypad_operators_keep_half_width_and_raw_input_order() {
    for mode in NumpadDigit::ALL {
        let mut router = router_with(RouterConfig {
            numpad_digit: mode,
            ..RouterConfig::default()
        });
        type_letters(&mut router, "gpt");
        press(
            &mut router,
            KeyEvent::new(0x6D, Some('-'), KeyModifiers::default()),
        );
        press(&mut router, keypad(6, KeyModifiers::default()));
        type_letters(&mut router, "gpt");
        let (_, commit, frame) = press(
            &mut router,
            KeyEvent::new(0x20, Some(' '), KeyModifiers::default()),
        );
        assert_eq!(commit.as_deref(), Some("gpt-6gpt "));
        assert!(frame.is_empty());
    }
}

#[test]
fn question_mode_without_cloud_candidates_does_not_append_digits() {
    let mut router = router_asking_with(RouterConfig {
        numpad_digit: NumpadDigit::Direct,
        ..RouterConfig::default()
    });
    let (_, _, before) = type_letters(&mut router, "unihao");
    let (_, commit, after) = press(&mut router, keypad(1, KeyModifiers::default()));
    assert_eq!(commit, None);
    assert_eq!(preedit(&after), preedit(&before));
}

#[test]
fn unicode_entry_still_receives_keypad_digits() {
    let mut router = direct_router();
    type_letters(&mut router, "u");
    press(&mut router, keypad(4, KeyModifiers::default()));
    type_letters(&mut router, "e");
    press(&mut router, keypad(0, KeyModifiers::default()));
    let (_, _, frame) = press(&mut router, keypad(0, KeyModifiers::default()));
    assert_eq!(frame.candidates.items[0].text, "一");
}

#[test]
fn numlock_off_navigation_does_not_insert_a_digit() {
    let mut router = direct_router();
    let (_, _, before) = type_letters(&mut router, "nihao");
    let (_, commit, after) = press(
        &mut router,
        KeyEvent::new(0x25, None, KeyModifiers::default()),
    );
    assert_eq!(commit, None);
    assert_eq!(router.engine_mut().composition().text(), "nihao");
    assert_ne!(after.cursor, before.cursor);
}
