//! 回放游戏没有光标位置、普通应用以及提交/失焦的候选显示路径；不启动游戏或云服务。

use crate::dispatch::{CandidateSink, RenderSettings, Router, RouterConfig};
use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_platform::protocol::{
    ClientMessage, Frame, KeyEvent, KeyModifiers, PROTOCOL_VERSION, ScreenRect, SessionId,
};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Capture {
    game: usize,
    normal: usize,
    hides: usize,
}

impl CandidateSink for Arc<Mutex<Capture>> {
    fn show(&self, _: Frame, _: qingjian_render::Mode, _: ScreenRect) {
        self.lock().unwrap().normal += 1;
    }
    fn show_game(&self, _: Frame, _: qingjian_render::Mode, _: &str) {
        self.lock().unwrap().game += 1;
    }
    fn hide(&self) {
        self.lock().unwrap().hides += 1;
    }
    fn configure(&self, _: RenderSettings) {}
}

fn compose(game_mode: bool, app: &str) -> (Router, Arc<Mutex<Capture>>) {
    let engine = Engine::new(Dictionary::parse("你\tni\t100\n").unwrap());
    let mut router = Router::new(
        engine,
        RouterConfig {
            game_mode,
            ..Default::default()
        },
    );
    let capture = Arc::new(Mutex::new(Capture::default()));
    router.set_candidate_sink(Box::new(capture.clone()));
    router.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: Some(app.into()),
        protocol: PROTOCOL_VERSION,
    });
    router.handle(ClientMessage::SyncMode {
        session: SessionId(1),
    });
    for c in "ni".chars() {
        router.handle(ClientMessage::Key {
            session: SessionId(1),
            event: KeyEvent::new(c as u32, Some(c), KeyModifiers::default()),
        });
    }
    (router, capture)
}

#[test]
fn game_candidates_do_not_require_caret_and_clear_on_commit() {
    let (mut router, capture) = compose(true, "DOTA2.EXE");
    assert!(capture.lock().unwrap().game > 0);
    assert_eq!(capture.lock().unwrap().normal, 0);
    let before = capture.lock().unwrap().hides;
    router.handle(ClientMessage::Key {
        session: SessionId(1),
        event: KeyEvent::new(32, Some(' '), KeyModifiers::default()),
    });
    assert!(capture.lock().unwrap().hides > before);
}

#[test]
fn ordinary_apps_and_daily_mode_still_require_caret() {
    for (game, app) in [(true, "notepad.exe"), (false, "dota2.exe")] {
        let (mut router, capture) = compose(game, app);
        assert_eq!(capture.lock().unwrap().game, 0);
        assert_eq!(capture.lock().unwrap().normal, 0);
        router.handle(ClientMessage::PositionCandidates {
            session: SessionId(1),
            rect: ScreenRect {
                left: 100,
                top: 100,
                right: 101,
                bottom: 120,
            },
        });
        assert!(capture.lock().unwrap().normal > 0);
    }
}

#[test]
fn game_candidates_clear_on_focus_loss_and_ime_switch() {
    for message in [
        ClientMessage::Commit {
            session: SessionId(1),
        },
        ClientMessage::ImeSwitched {
            session: SessionId(1),
        },
    ] {
        let (mut router, capture) = compose(true, "dota2.exe");
        let before = capture.lock().unwrap().hides;
        router.handle(message);
        assert!(capture.lock().unwrap().hides > before);
    }
}
