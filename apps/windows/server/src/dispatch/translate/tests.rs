//! 翻译等待和失败不提交空文本；长预览接受时仍提交全文。
use super::Translation;
use crate::dispatch::{Router, RouterConfig};
use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_platform::protocol::{KeyEvent, KeyModifiers, ServerMessage, SessionId};

fn router() -> Router {
    Router::new(Engine::new(Dictionary::default()), RouterConfig::default())
}
fn enter() -> KeyEvent {
    KeyEvent::new(0x0D, None, KeyModifiers::default())
}
#[test]
fn accepting_pending_review_keeps_waiting_without_a_commit() {
    let mut router = router();
    router.translation = Some(Translation {
        result: None,
        failed: false,
    });
    let reply = router.handle_translation_review(SessionId(1), &enter());
    assert!(matches!(
        reply,
        ServerMessage::KeyResult { commit: None, .. }
    ));
    assert!(router.translation.is_some());
}
#[test]
fn failed_review_keeps_original_and_can_be_dismissed() {
    let mut router = router();
    router.translation = Some(Translation {
        result: None,
        failed: true,
    });
    let frame = router.translation_frame(router.translation.as_ref().unwrap());
    assert!(frame.candidates.items[0].text.contains("原文已保留"));
    let reply = router.handle_translation_review(SessionId(1), &enter());
    assert!(matches!(
        reply,
        ServerMessage::KeyResult { commit: None, .. }
    ));
    assert!(router.translation.is_none());
}
#[test]
fn preview_is_short_but_accept_commits_every_character() {
    let mut router = router();
    let complete = format!(
        "  {}\n\n{}  ",
        "English paragraph. ".repeat(100),
        "日本語。".repeat(100)
    );
    router.translation = Some(Translation {
        result: Some(complete.clone()),
        failed: false,
    });
    let frame = router.translation_frame(router.translation.as_ref().unwrap());
    assert!(frame.candidates.items[0].text.chars().count() < 250);
    let reply = router.handle_translation_review(SessionId(1), &enter());
    assert!(matches!(reply, ServerMessage::KeyResult{commit:Some(text), ..} if text==complete));
    assert!(router.translation.is_none());
}
