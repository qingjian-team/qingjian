//! 备用选区不写应用、不自动碰剪贴板，等待、取消和失败均不得泄漏确认键。

use std::sync::atomic::Ordering;

use qingjian_platform::protocol::{
    ClientMessage, KeyEvent, KeyModifiers, KeyOutcome, ScreenRect, ServerMessage, SessionId,
};

use crate::dispatch::Router;
use crate::dispatch::translate::reading::ReadJob;

fn fallback() -> Router {
    let mut router = super::translated_selection(false);
    router.end_translation();
    let Some(ServerMessage::RequestSelection { request, .. }) = router.handle(ClientMessage::Key {
        session: SessionId(1),
        event: KeyEvent::new(
            84,
            Some('t'),
            KeyModifiers {
                ctrl: true,
                alt: true,
                ..Default::default()
            },
        ),
    }) else {
        panic!("selection request")
    };
    let reply = router.handle(ClientMessage::SelectionFallback {
        session: SessionId(1),
        request,
        process: 1,
        window: 1,
        text: "你好。".into(),
        rect: ScreenRect {
            left: 0,
            top: 0,
            right: 1,
            bottom: 1,
        },
    });
    assert!(matches!(
        reply,
        Some(ServerMessage::SelectionReview { active: true, .. })
    ));
    router.selection_copy = |text| {
        assert_eq!(text, "Hello.");
        Ok(())
    };
    router
}

fn key(router: &mut Router, vk: u32) -> Option<ServerMessage> {
    router.handle(ClientMessage::Key {
        session: SessionId(1),
        event: KeyEvent::new(vk, None, KeyModifiers::default()),
    })
}

#[test]
fn fallback_waits_for_confirmation_and_never_returns_a_document_commit() {
    let mut router = fallback();
    assert!(matches!(
        key(&mut router, 13),
        Some(ServerMessage::SelectionReview { active: true, .. })
    ));
    assert!(router.translation.is_some());
    router.poll_prediction();
    assert!(router.translation.as_ref().unwrap().result.is_some());
    assert!(matches!(
        key(&mut router, 13),
        Some(ServerMessage::SelectionReview { active: true, .. })
    ));
    let translation = router.translation.as_ref().unwrap();
    assert!(translation.copied);
    assert!(translation.result.is_none());
    let shown = router.self_drawn_frame();
    assert!(shown.candidates.items[0].text.contains("已复制"));
    assert!(router.engine.composition().is_empty());
    assert!(matches!(
        key(&mut router, 13),
        Some(ServerMessage::SelectionReview { active: false, .. })
    ));
}

#[test]
fn clipboard_failure_keeps_translation_available_for_retry() {
    let mut router = fallback();
    router.poll_prediction();
    router.selection_copy = |_| Err("剪贴板被占用");
    key(&mut router, 13);
    let translation = router.translation.as_ref().unwrap();
    assert!(!translation.copied);
    assert_eq!(translation.result.as_deref(), Some("Hello."));
    assert_eq!(translation.status.as_deref(), Some("剪贴板被占用"));
    router.selection_copy = |_| Ok(());
    key(&mut router, 13);
    assert!(router.translation.as_ref().unwrap().copied);
}

#[test]
fn escape_cancels_reading_and_late_results_cannot_reenter_the_engine() {
    let mut router = fallback();
    let (job, sender, cancelled) = ReadJob::pending();
    router.translation.as_mut().unwrap().reading = Some(job);
    router.selection_copy = |_| panic!("Escape must not copy");
    assert!(matches!(
        key(&mut router, 27),
        Some(ServerMessage::KeyResult {
            commit: None,
            outcome: KeyOutcome::Consumed,
            ..
        })
    ));
    assert!(cancelled.load(Ordering::Relaxed));
    assert!(sender.send(Ok("迟到的文字".into())).is_err());
    router.poll_prediction();
    assert!(router.translation.is_none());
}

#[test]
fn reading_failure_is_visible_and_confirmation_stays_consumed() {
    let mut router = fallback();
    router.engine.cancel_prediction();
    let (job, sender, _) = ReadJob::pending();
    router.translation.as_mut().unwrap().reading = Some(job);
    sender.send(Err("当前应用未提供可读选区（UIA）")).unwrap();
    router.poll_prediction();
    assert!(
        router.self_drawn_frame().candidates.items[0]
            .text
            .contains("未提供可读选区")
    );
    assert!(matches!(
        key(&mut router, 13),
        Some(ServerMessage::SelectionReview { active: true, .. })
    ));
    assert!(router.translation.as_ref().unwrap().result.is_none());
}

#[test]
fn wrong_request_private_session_and_old_client_do_not_start_fallback() {
    for scenario in 0..3 {
        let mut router = fallback();
        router.end_translation();
        router.pending_selection = Some(77);
        if scenario == 1 {
            router.set_privacy(SessionId(1), true);
        }
        if scenario == 2 {
            router.sessions.get_mut(&SessionId(1)).unwrap().protocol = 9;
        }
        let reply = router.handle(ClientMessage::SelectionFallback {
            session: SessionId(1),
            request: if scenario == 0 { 76 } else { 77 },
            process: 0,
            window: 0,
            text: "不得处理".into(),
            rect: ScreenRect {
                left: 0,
                top: 0,
                right: 1,
                bottom: 1,
            },
        });
        assert!(matches!(
            reply,
            Some(ServerMessage::SelectionReview { active: false, .. })
        ));
        assert!(router.translation.is_none());
    }
}

#[test]
fn paste_command_closes_review_and_passes_through_without_an_ime_commit() {
    let mut router = fallback();
    router.poll_prediction();
    key(&mut router, 13);
    let reply = router.handle(ClientMessage::Key {
        session: SessionId(1),
        event: KeyEvent::new(
            86,
            Some('v'),
            KeyModifiers {
                ctrl: true,
                ..Default::default()
            },
        ),
    });
    assert!(matches!(
        reply,
        Some(ServerMessage::KeyResult {
            commit: None,
            outcome: KeyOutcome::Passthrough,
            ..
        })
    ));
    assert!(router.translation.is_none());
}
