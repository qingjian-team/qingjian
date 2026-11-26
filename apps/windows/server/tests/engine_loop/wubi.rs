//! 纯五笔选词键和上屏后联想的按键、点击与会话闭环。

use qingjian_core::sentence::LanguageModel;
use qingjian_core::{CodeScheme, Engine};
use qingjian_dictionary::{CodeTable, Dictionary};
use qingjian_platform::{CommitKey, CommitKeys};
use qingjian_render::HitTarget;

use crate::support::{
    ClientMessage, KeyEvent, KeyOutcome, Router, RouterConfig, SESSION, ServerMessage, SessionId,
    candidate_texts, digit, letter, open_session, press, punct, type_letters,
};

struct FollowModel;

impl LanguageModel for FollowModel {
    fn log_prob(&self, _previous: Option<&str>, _word: &str) -> Option<f64> {
        None
    }

    fn successors(&self, previous: &str, limit: usize) -> Vec<String> {
        let words: &[&str] = match previous {
            "开发" => &["软件", "人员", "工具"],
            "软件" => &["设计"],
            "人员" => &["管理"],
            _ => &[],
        };
        words
            .iter()
            .take(limit)
            .map(|word| (*word).to_owned())
            .collect()
    }
}

fn router(config: RouterConfig, mixed: bool) -> Router {
    let table = CodeTable::parse(
        "首选\tgg\t100\n次选\tgg\t90\n第三\tgg\t80\n第四\tgg\t70\n第五\tgg\t60\n第六\tgg\t50\n开发\tgant\t100\n",
    ).unwrap();
    let mut scheme = CodeScheme::new("wubi", table);
    scheme.code_length = 4;
    scheme.auto_commit = true;
    scheme.top = true;
    let mut engine = Engine::new(Dictionary::default()).with_language_model(Box::new(FollowModel));
    engine.set_code_scheme(Some(scheme));
    engine.set_phonetic(mixed);
    let mut router = Router::new(engine, config);
    open_session(&mut router, SESSION, None);
    router
}

#[test]
fn punctuation_keys_select_the_current_page_slots() {
    for (symbol, text) in [(';', "第五"), ('\'', "第六")] {
        let mut router = router(
            RouterConfig {
                page_size: 3,
                ..RouterConfig::default()
            },
            false,
        );
        type_letters(&mut router, "gg");
        let (_, _, frame) = press(&mut router, punct(']'));
        assert_eq!(frame.page, 1);
        assert_eq!(press(&mut router, punct(symbol)).1.as_deref(), Some(text));
    }
}

#[test]
fn wubi_shortcuts_leave_mixed_input_and_disabled_settings_alone() {
    for (config, mixed) in [
        (RouterConfig::default(), true),
        (
            RouterConfig {
                wubi_candidate_keys: false,
                ..RouterConfig::default()
            },
            false,
        ),
    ] {
        let mut router = router(config, mixed);
        type_letters(&mut router, "gg");
        let (_, text, _) = press(&mut router, punct(';'));
        assert_ne!(text.as_deref(), Some("次选"));
    }
    let mut router = router(RouterConfig::default(), false);
    let (_, text, _) = press(&mut router, punct(';'));
    assert_eq!(text.as_deref(), Some("；"));
}

#[test]
fn wubi_shortcuts_and_follow_ups_can_be_enabled_independently() {
    let mut only_shortcuts = router(
        RouterConfig {
            wubi_follow_ups: false,
            ..RouterConfig::default()
        },
        false,
    );
    type_letters(&mut only_shortcuts, "gg");
    assert_eq!(
        press(&mut only_shortcuts, punct(';')).1.as_deref(),
        Some("次选")
    );
    let (_, text, frame) = type_letters(&mut only_shortcuts, "gant");
    assert_eq!(text.as_deref(), Some("开发"));
    assert!(frame.is_empty());

    let mut only_follow_ups = router(
        RouterConfig {
            wubi_candidate_keys: false,
            ..RouterConfig::default()
        },
        false,
    );
    assert!(
        type_letters(&mut only_follow_ups, "gant")
            .2
            .is_code_follow_up()
    );
    let space = KeyEvent::new(0x20, Some(' '), Default::default());
    assert_eq!(
        press(&mut only_follow_ups, space).1.as_deref(),
        Some("软件")
    );
}

#[test]
fn full_code_and_follow_up_commit_use_the_same_local_model() {
    let mut router = router(RouterConfig::default(), false);
    let (_, text, frame) = type_letters(&mut router, "gant");
    assert_eq!(text.as_deref(), Some("开发"));
    assert!(frame.is_code_follow_up());
    assert_eq!(candidate_texts(&frame), ["软件", "人员", "工具"]);
    assert_eq!(frame.highlight, 0);
    let (_, text, frame) = press(&mut router, punct(';'));
    assert_eq!(text.as_deref(), Some("人员"));
    assert_eq!(candidate_texts(&frame), ["管理"]);
    assert_eq!(frame.highlight, 0);
    router.handle_click(HitTarget::Candidate(0));
    let Some(ServerMessage::Update { frame, commit, .. }) =
        router.handle(ClientMessage::Poll { session: SESSION })
    else {
        panic!("expected Update");
    };
    assert_eq!(commit.as_deref(), Some("管理"));
    assert!(frame.is_empty());
}

#[test]
fn a_fresh_space_selects_the_first_follow_up_but_a_held_space_does_not() {
    let mut router = router(RouterConfig::default(), false);
    router.engine_mut().set_code_rules(false, true);
    type_letters(&mut router, "gant");

    let space = KeyEvent::new(0x20, Some(' '), Default::default());
    let (_, text, frame) = press(&mut router, space);
    assert_eq!(text.as_deref(), Some("开发"));
    assert!(frame.is_code_follow_up());

    let (outcome, text, frame) = press(&mut router, space.with_repeat(true));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(text.is_none());
    assert_eq!(candidate_texts(&frame)[frame.highlight], "软件");

    let (_, text, frame) = press(&mut router, space);
    assert_eq!(text.as_deref(), Some("软件"));
    assert_eq!(candidate_texts(&frame), ["设计"]);
    let (outcome, text, frame) = press(&mut router, space.with_repeat(true));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(text.is_none() && frame.is_code_follow_up());

    assert_eq!(press(&mut router, space).1.as_deref(), Some("设计"));
    let (outcome, text, frame) = press(&mut router, space);
    assert_eq!(outcome, KeyOutcome::Passthrough);
    assert!(text.is_none() && frame.is_empty());
}

#[test]
fn space_remains_native_if_the_normal_candidate_confirmation_key_is_rebound() {
    let mut router = router(
        RouterConfig {
            commit_keys: CommitKeys {
                candidate: CommitKey::Symbol(';'),
                raw: CommitKey::ENTER,
            },
            ..RouterConfig::default()
        },
        false,
    );
    assert!(type_letters(&mut router, "gant").2.is_code_follow_up());
    let space = KeyEvent::new(0x20, Some(' '), Default::default());
    let (outcome, text, frame) = press(&mut router, space);
    assert_eq!(outcome, KeyOutcome::Passthrough);
    assert!(text.is_none() && frame.is_empty());
}

#[test]
fn restarting_input_or_dismissing_the_follow_up_keeps_native_keys() {
    let mut router = router(RouterConfig::default(), false);
    let nav = |key| KeyEvent::new(key, None, Default::default());
    for key in [0x08, 0x0d, 0x25] {
        assert!(type_letters(&mut router, "gant").2.is_code_follow_up());
        let (outcome, commit, frame) = press(&mut router, nav(key));
        assert_eq!(outcome, KeyOutcome::Passthrough);
        assert!(commit.is_none() && frame.is_empty());
    }
    type_letters(&mut router, "gant");
    let (outcome, _, frame) = press(&mut router, nav(0x1b));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(frame.is_empty());
    type_letters(&mut router, "gant");
    let (_, _, frame) = press(&mut router, letter('g'));
    assert!(!frame.is_code_follow_up());
    assert!(!frame.preedit.is_empty());
}

#[test]
fn follow_ups_do_not_cross_privacy_or_focus_boundaries() {
    let mut router = router(RouterConfig::default(), false);
    type_letters(&mut router, "gant");
    router.handle(ClientMessage::Privacy {
        session: SESSION,
        private: true,
    });
    assert!(type_letters(&mut router, "gant").2.is_empty());
    router.handle(ClientMessage::Privacy {
        session: SESSION,
        private: false,
    });
    assert!(type_letters(&mut router, "gant").2.is_code_follow_up());
    let other = SessionId(2);
    open_session(&mut router, other, None);
    let (_, text, frame) = crate::support::key_result(router.handle(ClientMessage::Key {
        session: other,
        event: digit(1),
    }));
    assert!(text.is_none() && frame.is_empty());
}
