//! 鼠标点候选窗口：上屏的字挂到焦点会话上，下一拍轮询（或抢先来的键）带给 DLL；老协议的 DLL 不收点击。

use qingjian_render::HitTarget;

use crate::support::*;

fn poll(router: &mut Router, session: SessionId) -> (Frame, Option<String>) {
    match router.handle(ClientMessage::Poll { session }) {
        Some(ServerMessage::Update { frame, commit, .. }) => (frame, commit),
        other => panic!("expected Update, got {other:?}"),
    }
}

#[test]
fn clicking_a_candidate_commits_it_on_the_next_poll() {
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let slot = slot_of(&frame, "你好");
    router.handle_click(HitTarget::Candidate(slot as usize - 1));
    let (after, commit) = poll(&mut router, SESSION);
    assert_eq!(commit.as_deref(), Some("你好"));
    assert!(after.is_empty());
    // 交出去一次就没了
    assert_eq!(poll(&mut router, SESSION).1, None);
}

#[test]
fn clicking_a_translation_commits_that_sense() {
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let slot = slot_of(&frame, "你好");
    router.handle_click(HitTarget::Translation {
        candidate: slot as usize - 1,
        sense: 0,
    });
    assert_eq!(poll(&mut router, SESSION).1.as_deref(), Some("hello"));
}

#[test]
fn a_key_before_the_poll_carries_the_clicked_text() {
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let slot = slot_of(&frame, "你好");
    router.handle_click(HitTarget::Candidate(slot as usize - 1));
    let (_, commit, frame) = press(&mut router, letter('w'));
    assert_eq!(commit.as_deref(), Some("你好"));
    assert_eq!(preedit(&frame), "w");
    assert_eq!(poll(&mut router, SESSION).1, None);
}

#[test]
fn old_dlls_do_not_take_clicks() {
    let mut router = router();
    let old = SessionId(2);
    router.handle(ClientMessage::OpenSession {
        session: old,
        app: None,
        protocol: PROTOCOL_VERSION - 1,
    });
    for c in "nihao".chars() {
        press_in(&mut router, old, letter(c));
    }
    router.handle_click(HitTarget::Candidate(0));
    let (frame, commit) = poll(&mut router, old);
    assert_eq!(commit, None);
    assert_eq!(preedit(&frame), "ni'hao");
}
