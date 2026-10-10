//! 鼠标与常用翻页键的回归：页界、保留组句、当前页选择和符号输入。

use qingjian_platform::protocol::{
    ClientMessage, KeyModifiers, KeyOutcome, ServerMessage, SessionId,
};
use qingjian_render::HitTarget;

use super::{compose, key, router};

#[test]
fn minus_equals_turn_pages_and_select_from_the_current_page() {
    for size in [1, 4, 5, 9] {
        let mut router = router(size);
        router.config.page_keys = ('-', '=');
        let normal = KeyModifiers::default();
        compose(&mut router, "qq", normal);
        let preedit = router.self_drawn_frame().preedit;
        assert_eq!(key(&mut router, 0xBD, Some('-'), normal).2.page, 0);
        let last = 9_usize.div_ceil(size) - 1;
        for page in 1..=last {
            let result = key(&mut router, 0xBB, Some('='), normal);
            assert_eq!(result.0, KeyOutcome::Consumed);
            assert_eq!(result.1, None);
            assert_eq!((result.2.page, result.2.highlight), (page, 0));
            assert_eq!(result.2.preedit, preedit);
        }
        assert_eq!(key(&mut router, 0xBB, Some('='), normal).2.page, last);
        let expected = router.self_drawn_frame().candidates.items[0].text.clone();
        assert_eq!(key(&mut router, 0x31, Some('1'), normal).1, Some(expected));
    }
}

#[test]
fn clicks_preserve_composition_and_pending_commit_and_poll_the_new_page() {
    let mut router = router(4);
    let normal = KeyModifiers::default();
    compose(&mut router, "qq", normal);
    router.highlight = 1;
    router.handle_page_click(-1);
    assert_eq!(router.highlight, 1);
    let preedit = router.self_drawn_frame().preedit;
    router.clicked = Some((SessionId(1), "待上屏".to_owned()));
    router.handle_page_click(1);
    assert_eq!(router.highlight, 4);
    assert!(router.navigated);
    assert_eq!(router.self_drawn_frame().preedit, preedit);
    match router
        .handle(ClientMessage::Poll {
            session: SessionId(1),
        })
        .unwrap()
    {
        ServerMessage::Update { frame, commit, .. } => {
            assert_eq!((frame.page, frame.highlight), (1, 0));
            assert_eq!(commit.as_deref(), Some("待上屏"));
        }
        _ => panic!("poll result"),
    }
    router.handle_page_click(1);
    router.handle_page_click(1);
    assert_eq!(router.self_drawn_frame().page, 2);
    router.handle_page_click(-1);
    let expected = router.self_drawn_frame().candidates.items[0].text.clone();
    router.handle_click(HitTarget::Candidate(0));
    assert_eq!(router.take_clicked(SessionId(1)), Some(expected));
    assert!(router.self_drawn_frame().preedit.is_empty());
    router.handle_page_click(1);
    assert!(router.self_drawn_frame().preedit.is_empty());
}

#[test]
fn paging_preserves_configured_keys_and_english_candidates() {
    let mut router = router(1);
    let english = KeyModifiers {
        english_mode: true,
        ..Default::default()
    };
    router.config.page_keys = ('-', '=');
    compose(&mut router, "hel", english);
    assert_eq!(key(&mut router, 0xBB, Some('='), english).2.page, 1);
    let previous = key(&mut router, 0xBD, Some('-'), english);
    assert_eq!(previous.1, None);
    assert_eq!(previous.2.page, 0);
    assert_eq!(router.engine.composition().text(), "hel");
    key(&mut router, 0x1B, None, english);
    router.config.page_keys = (',', '.');
    let normal = KeyModifiers::default();
    compose(&mut router, "qq", normal);
    assert_eq!(key(&mut router, 0xBE, Some('.'), normal).2.page, 1);
    assert_eq!(key(&mut router, 0xBC, Some(','), normal).2.page, 0);
    router.config.page_keys = ('[', ']');
    assert_eq!(key(&mut router, 0xDD, Some(']'), normal).2.page, 1);
    assert_eq!(key(&mut router, 0xDB, Some('['), normal).2.page, 0);
}

#[test]
fn symbols_keep_their_literal_role_without_candidates_and_in_expressions() {
    let mut router = router(1);
    let normal = KeyModifiers::default();
    for (code, c) in [(0xBD, '-'), (0xBB, '=')] {
        assert_eq!(
            key(&mut router, code, Some(c), normal).0,
            KeyOutcome::Passthrough
        );
    }
    compose(&mut router, "zzzz", normal);
    key(&mut router, 0xBD, Some('-'), normal);
    key(&mut router, 0xBB, Some('='), normal);
    assert_eq!(
        key(&mut router, 0x0D, None, normal).1.as_deref(),
        Some("zzzz-=")
    );
    router.config.page_keys = ('-', '=');
    compose(&mut router, "v1", normal);
    key(&mut router, 0xBD, Some('-'), normal);
    compose(&mut router, "2", normal);
    assert_eq!(
        key(&mut router, 0x0D, None, normal).1.as_deref(),
        Some("v1-2")
    );
    compose(&mut router, "qq", normal);
    key(
        &mut router,
        0xBB,
        Some('+'),
        KeyModifiers {
            shift: true,
            ..normal
        },
    );
    assert_eq!(router.engine.composition().text(), "qq+");
}

#[test]
fn clicks_ignore_missing_focus_empty_candidates_and_invalid_steps() {
    let mut router = router(1);
    router.handle_page_click(1);
    let normal = KeyModifiers::default();
    compose(&mut router, "qq", normal);
    for step in [0, -2, 2, isize::MAX] {
        router.handle_page_click(step);
        assert_eq!(router.highlight, 0);
    }
    router.focused = None;
    router.handle_page_click(1);
    assert_eq!(router.highlight, 0);
}

#[test]
fn zhuyin_minus_keeps_its_input_role_when_configured_for_paging() {
    let mut router = router(1);
    let normal = KeyModifiers::default();
    router.engine.set_zhuyin_mode(true);
    router.config.page_keys = ('-', '=');
    compose(&mut router, "su", normal);
    key(&mut router, 0xBD, Some('-'), normal);
    assert_eq!(router.engine.composition().text(), "su-");
}
