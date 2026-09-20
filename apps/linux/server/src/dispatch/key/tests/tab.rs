//! Tab、整句补全与末页边界。
use super::support::{compose, key, router};
use qingjian_platform::protocol::{KeyModifiers, KeyOutcome};
#[test]
fn tab_and_backtab_page_boundaries_and_current_page_selection() {
    for size in [1, 4, 5, 9] {
        let mut router = router(size);
        let normal = KeyModifiers::default();
        let shift = KeyModifiers {
            shift: true,
            ..normal
        };
        assert_eq!(key(&mut router, 9, None, normal).0, KeyOutcome::Passthrough);
        assert_eq!(key(&mut router, 9, None, shift).0, KeyOutcome::Passthrough);
        compose(&mut router, "qq", normal);
        assert_eq!(key(&mut router, 9, None, shift).2.page, 0);
        let last = 9_usize.div_ceil(size) - 1;
        for page in 1..=last {
            let result = key(&mut router, 9, None, normal);
            assert_eq!(result.0, KeyOutcome::Consumed);
            assert_eq!((result.2.page, result.2.highlight), (page, 0));
        }
        let frame = key(&mut router, 9, None, normal).2;
        assert_eq!(frame.page, last);
        let expected = frame.candidates.items[0].text.clone();
        assert_eq!(
            key(&mut router, b'1' as u32, Some('1'), normal).1,
            Some(expected)
        );
        compose(&mut router, "qq", normal);
        key(&mut router, 0x22, None, normal);
        assert_eq!(key(&mut router, 9, None, shift).2.page, 0);
        assert_eq!(key(&mut router, 0x21, None, normal).2.page, 0);
    }
}
#[test]
fn shift_tab_precedes_prediction_and_english_commit() {
    let mut router = router(1);
    let normal = KeyModifiers::default();
    compose(&mut router, "qq", normal);
    router.sentence = Some("可控补全".into());
    let result = key(
        &mut router,
        9,
        None,
        KeyModifiers {
            shift: true,
            ..normal
        },
    );
    assert_eq!(result.1, None);
    assert_eq!(router.sentence.as_deref(), Some("可控补全"));
    assert_eq!(
        key(&mut router, 9, None, normal).1.as_deref(),
        Some("可控补全")
    );
    let english = KeyModifiers {
        english_mode: true,
        ..normal
    };
    compose(&mut router, "hel", english);
    key(&mut router, 0x22, None, english);
    let previous = key(
        &mut router,
        9,
        None,
        KeyModifiers {
            shift: true,
            ..english
        },
    );
    assert_eq!(previous.1, None);
    assert_eq!(previous.2.page, 0);
    let expected = previous.2.candidates.items[previous.2.highlight]
        .text
        .clone();
    assert_eq!(key(&mut router, 9, None, english).1, Some(expected));
}
#[test]
fn tab_with_raw_input_and_no_candidates_is_consumed_without_commit() {
    let mut router = router(5);
    compose(&mut router, "zzzz", KeyModifiers::default());
    let result = key(&mut router, 9, None, KeyModifiers::default());
    assert_eq!(result.0, KeyOutcome::Consumed);
    assert_eq!(result.1, None);
    assert_eq!(result.2.page, 0);
}

#[test]
fn paging_at_the_boundary_returns_to_first_slot_on_the_same_page() {
    let mut router = router(5);
    let normal = KeyModifiers::default();
    compose(&mut router, "qq", normal);
    key(&mut router, 0x28, None, normal);
    assert_eq!(key(&mut router, 0x21, None, normal).2.highlight, 0);
    key(&mut router, 0x22, None, normal);
    key(&mut router, 0x28, None, normal);
    assert_eq!(key(&mut router, 0x22, None, normal).2.highlight, 0);
}
