//! 空 TSF 选区应当终止请求，并允许下一次正常选区翻译。

use std::sync::atomic::Ordering;

use qingjian_platform::protocol::{
    ClientMessage, KeyEvent, KeyModifiers, KeyOutcome, ServerMessage,
};

use super::{SELECTION_RECT, SESSION, request_selection, send_selection, translation_router};
use crate::dispatch::RouterConfig;

#[test]
fn blank_selection_clears_pending_request_without_translation_or_commit() {
    for text in ["", " \t\r\n\u{3000}"] {
        let (mut router, submissions) = translation_router(Some("Hello."), RouterConfig::default());
        let blank_request = request_selection(&mut router);
        assert_eq!(router.pending_selection, Some(blank_request));

        assert!(matches!(
            send_selection(&mut router, blank_request, text),
            ServerMessage::KeyResult {
                session: SESSION,
                outcome: KeyOutcome::Consumed,
                commit: None,
                frame,
                ..
            } if frame.is_empty()
        ));
        assert!(router.pending_selection.is_none());
        assert!(router.translation.is_none());
        assert!(router.last_rect.is_none());
        assert!(matches!(
            router.handle(ClientMessage::Poll { session: SESSION }),
            Some(ServerMessage::Update {
                session: SESSION,
                commit: None,
                frame,
            }) if frame.is_empty()
        ));
        assert_eq!(submissions.load(Ordering::Relaxed), 0);

        let next_request = request_selection(&mut router);
        assert_ne!(next_request, blank_request);
        assert!(matches!(
            send_selection(&mut router, blank_request, "过期选区"),
            ServerMessage::KeyResult {
                outcome: KeyOutcome::Passthrough,
                commit: None,
                frame,
                ..
            } if frame.is_empty()
        ));
        assert_eq!(router.pending_selection, Some(next_request));
        assert_eq!(submissions.load(Ordering::Relaxed), 0);

        assert!(matches!(
            send_selection(&mut router, next_request, "你好。"),
            ServerMessage::KeyResult {
                outcome: KeyOutcome::Consumed,
                commit: None,
                ..
            }
        ));
        assert_eq!(submissions.load(Ordering::Relaxed), 1);
        assert_eq!(router.last_rect, Some(SELECTION_RECT));
        assert!(matches!(
            router.handle(ClientMessage::Poll { session: SESSION }),
            Some(ServerMessage::Update {
                session: SESSION,
                commit: None,
                frame,
            }) if frame.candidates.items.first().is_some_and(|item| item.text == "Hello.")
        ));
        assert!(matches!(
            router.handle(ClientMessage::Key {
                session: SESSION,
                event: KeyEvent::new(13, None, KeyModifiers::default()),
            }),
            Some(ServerMessage::KeyResult {
                outcome: KeyOutcome::Consumed,
                commit: Some(text),
                ..
            }) if text == "Hello."
        ));
    }
}
