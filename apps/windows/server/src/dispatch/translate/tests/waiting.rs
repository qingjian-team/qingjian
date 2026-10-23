//! 译文未就绪时确认只吃键，保留请求；旧 DLL 不接收新增协议消息。

use qingjian_platform::protocol::{
    ClientMessage, KeyEvent, KeyModifiers, KeyOutcome, ServerMessage, SessionId,
};

use crate::dispatch::Router;

#[test]
fn failed_translation_keeps_a_visible_status_and_never_commits_it() {
    let mut router = super::translated_selection_answer(false, None);
    router.poll_prediction();
    let frame = router.translation_frame(router.translation.as_ref().unwrap());
    assert!(frame.candidates.items[0].text.contains("未获得可用译文"));
    assert!(matches!(
        confirm(&mut router, 13, None),
        ServerMessage::SelectionReview { active: true, .. }
    ));
    assert!(router.translation.is_some());
    assert!(matches!(
        confirm(&mut router, 27, None),
        ServerMessage::KeyResult { commit: None, .. }
    ));
    assert!(router.translation.is_none());
}

fn confirm(router: &mut Router, virtual_key: u32, character: Option<char>) -> ServerMessage {
    router
        .handle(ClientMessage::Key {
            session: SessionId(1),
            event: KeyEvent::new(virtual_key, character, KeyModifiers::default()),
        })
        .unwrap()
}

#[test]
fn confirmation_while_waiting_preserves_review_and_later_commits() {
    let mut router = super::translated_selection(false);
    for (virtual_key, character) in [(13, None), (32, Some(' ')), (49, Some('1'))] {
        assert!(matches!(
            confirm(&mut router, virtual_key, character),
            ServerMessage::SelectionReview { active: true, .. }
        ));
        assert!(router.translation.as_ref().unwrap().result.is_none());
        assert!(router.engine.composition().is_empty());
    }
    router.poll_prediction();
    assert!(matches!(
        confirm(&mut router, 13, None),
        ServerMessage::KeyResult {
            outcome: KeyOutcome::Consumed,
            commit: Some(text),
            ..
        } if text == "Hello."
    ));
    assert!(router.translation.is_none());
}

#[test]
fn old_client_confirmation_while_waiting_keeps_supported_reply_and_pending_work() {
    let mut router = super::translated_selection(false);
    router.sessions.get_mut(&SessionId(1)).unwrap().protocol = 9;
    for (virtual_key, character) in [(13, None), (32, Some(' ')), (49, Some('1'))] {
        assert!(matches!(
            confirm(&mut router, virtual_key, character),
            ServerMessage::KeyResult {
                outcome: KeyOutcome::Consumed,
                commit: None,
                frame,
                ..
            } if frame.is_empty()
        ));
        assert!(router.translation.as_ref().unwrap().result.is_none());
    }
    router.poll_prediction();
    assert_eq!(
        router.translation.as_ref().unwrap().result.as_deref(),
        Some("Hello.")
    );
}

#[test]
fn escape_after_early_confirmation_still_discards_late_result() {
    let mut router = super::translated_selection(false);
    assert!(matches!(
        confirm(&mut router, 13, None),
        ServerMessage::SelectionReview { active: true, .. }
    ));
    assert!(matches!(
        confirm(&mut router, 27, None),
        ServerMessage::KeyResult {
            outcome: KeyOutcome::Consumed,
            commit: None,
            ..
        }
    ));
    assert!(router.translation.is_none());
    assert!(router.engine.poll_prediction().is_none());
}
