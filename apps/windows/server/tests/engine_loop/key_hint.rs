//! 键位提示：自绘候选窗随帧收到双拼键位；「卡住时显示」停顿到点由 `tick` 弹出，组句结束后重新计时。

use std::time::Duration;

use crate::support::*;
use qingjian_platform::KeyHintMode;

fn shuangpin_router(mode: KeyHintMode) -> (Router, RecordingCandidates) {
    let mut router = router_with(RouterConfig {
        scheme: Scheme::Shuangpin(ShuangpinScheme::Xiaohe),
        key_hint: mode,
        ..RouterConfig::default()
    });
    let sink = RecordingCandidates::default();
    router.set_candidate_sink(Box::new(sink.clone()));
    (router, sink)
}

fn position(router: &mut Router) {
    let _ = router.handle(ClientMessage::PositionCandidates {
        session: SESSION,
        rect: rect(),
    });
}

fn last_hint(sink: &RecordingCandidates) -> Option<KeyHintView> {
    sink.2.lock().unwrap().last().cloned().flatten()
}

#[test]
fn always_shows_the_shuangpin_keys_while_composing() {
    let (mut router, sink) = shuangpin_router(KeyHintMode::Always);
    type_letters(&mut router, "k");
    position(&mut router);
    let hint = last_hint(&sink).expect("组句时带着键位提示");
    assert!(matches!(hint.content, KeyHintContent::Shuangpin(_)));
}

#[test]
fn off_never_shows_keys() {
    let (mut router, sink) = shuangpin_router(KeyHintMode::Off);
    type_letters(&mut router, "k");
    position(&mut router);
    assert!(!sink.2.lock().unwrap().is_empty());
    assert_eq!(last_hint(&sink), None);
}

#[test]
fn auto_pops_up_after_a_pause_and_resets_after_commit() {
    let (mut router, sink) = shuangpin_router(KeyHintMode::Auto);
    type_letters(&mut router, "k");
    position(&mut router);
    assert_eq!(last_hint(&sink), None, "刚敲完不弹");
    assert!(router.next_tick() <= Duration::from_millis(700));
    std::thread::sleep(Duration::from_millis(750));
    router.tick();
    assert!(last_hint(&sink).is_some(), "停顿到点弹出");
    press(&mut router, letter(' '));
    type_letters(&mut router, "k");
    position(&mut router);
    assert_eq!(last_hint(&sink), None, "下一段组句重新计时");
}
