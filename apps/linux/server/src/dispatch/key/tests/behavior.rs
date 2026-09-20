//! 数字缺席、直输、英文候选与小键盘的实际上屏结果。
use super::support::{compose, key, router};
use qingjian_platform::protocol::{KeyModifiers, KeyOutcome};

#[test]
fn absent_slot_keeps_digit_in_buffer_and_raw_submission() {
    let normal = KeyModifiers::default();
    let mut router = router(5);
    compose(&mut router, "ni", normal);
    let result = key(&mut router, 0x36, Some('6'), normal);
    assert_eq!(result.0, KeyOutcome::Consumed);
    assert!(result.1.is_none());
    assert_eq!(key(&mut router, 13, None, normal).1.as_deref(), Some("ni6"));
    compose(&mut router, "qq", normal);
    key(&mut router, 9, None, normal);
    // 末页只有四项，第五个数字不能误选或丢失。
    key(&mut router, 0x35, Some('5'), normal);
    assert_eq!(key(&mut router, 13, None, normal).1.as_deref(), Some("qq5"));
}
#[test]
fn raw_segment_preserves_digit_and_space_exactly_once() {
    let mut router = router(5);
    let normal = KeyModifiers::default();
    compose(&mut router, "gpt-6", normal);
    let result = key(&mut router, 32, Some(' '), normal);
    assert_eq!(result.0, KeyOutcome::Consumed);
    assert_eq!(result.1.as_deref(), Some("gpt-6 "));
    assert!(result.2.is_empty());
}
#[test]
fn english_candidates_select_current_page_and_preserve_trailing_space() {
    let mut router = router(1);
    let english = KeyModifiers {
        english_mode: true,
        ..Default::default()
    };
    compose(&mut router, "hel", english);
    let second = key(&mut router, b']' as u32, Some(']'), english).2;
    assert_eq!(second.page, 1);
    assert_eq!(
        key(&mut router, 0x31, Some('1'), english).1,
        Some(second.candidates.items[0].text.clone())
    );
    compose(&mut router, "hel", english);
    let first = key(&mut router, 0, None, english).2.candidates.items[0]
        .text
        .clone();
    let result = key(&mut router, 32, Some(' '), english);
    assert_eq!(result.0, KeyOutcome::Consumed);
    assert_eq!(result.1, Some(first + " "));
    compose(&mut router, "hel", english);
    key(&mut router, 0x39, Some('9'), english);
    assert_eq!(
        key(&mut router, 13, None, english).1.as_deref(),
        Some("hel9")
    );
}
#[test]
fn keypad_punctuation_is_half_width_and_regular_punctuation_is_full_width() {
    let mut router = router(5);
    let normal = KeyModifiers::default();
    for (code, character) in [
        (0x6a, '*'),
        (0x6b, '+'),
        (0x6d, '-'),
        (0x6e, '.'),
        (0x6f, '/'),
    ] {
        let result = key(&mut router, code, Some(character), normal);
        assert_eq!(result.0, KeyOutcome::Passthrough);
        assert!(result.1.is_none());
    }
    assert_eq!(
        key(&mut router, b'.' as u32, Some('.'), normal)
            .1
            .as_deref(),
        Some("。")
    );
}
#[test]
fn shift_letter_configuration_does_not_change_caps_or_english() {
    let mut router = router(5);
    let shift = KeyModifiers {
        shift: true,
        ..Default::default()
    };
    assert_eq!(
        key(&mut router, b'N' as u32, Some('N'), shift).0,
        KeyOutcome::Passthrough
    );
    router.config.shift_letter_compose = true;
    router.engine.set_shift_letter_compose(true);
    compose(&mut router, "Ni", shift);
    assert_eq!(
        key(&mut router, 32, Some(' '), shift).1.as_deref(),
        Some("你")
    );
    let caps = KeyModifiers {
        caps: true,
        ..Default::default()
    };
    assert_eq!(
        key(&mut router, b'N' as u32, Some('N'), caps).1.as_deref(),
        Some("N")
    );
    assert!(router.engine.composition().is_empty());
}
