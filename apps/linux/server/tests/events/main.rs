//! 能力优先级、焦点边界和薄插件事件的业务决策。
mod support;
use self::support::{caps, compose, event, key, router};
use serde_json::json;

#[test]
fn capability_priority_discards_same_privacy_transitions_and_never_commits_old_text() {
    let mut router = router();
    assert_eq!(compose(&mut router, 1, "nihao")["outcome"], "Consumed");
    caps(&mut router, 1, true, false, false);
    assert!(router.is_private());
    assert_eq!(key(&mut router, 1, 13, None, false)["commit"], json!(null));
    compose(&mut router, 1, "nihao");
    caps(&mut router, 1, true, true, false);
    assert_eq!(
        key(&mut router, 1, 32, Some(' '), false)["outcome"],
        "Passthrough"
    );
    assert_eq!(
        event(
            &mut router,
            1,
            json!({"Deactivate": {"focus_out": false, "client_preedit": false, "capability_changed": false}})
        )["commit"],
        json!(null)
    );
    caps(&mut router, 1, false, false, true);
    assert_eq!(compose(&mut router, 1, "ni")["outcome"], "Passthrough");
    caps(&mut router, 1, false, false, false);
    assert_eq!(compose(&mut router, 1, "nihao")["outcome"], "Consumed");
    assert!(!router.is_private());
    assert_eq!(key(&mut router, 1, 32, Some(' '), false)["commit"], "你好");
}
#[test]
fn shift_click_and_deactivate_decisions_live_in_server() {
    let mut router = router();
    compose(&mut router, 1, "ni");
    assert_eq!(
        key(&mut router, 1, 16, None, false)["outcome"],
        "Passthrough"
    );
    assert_eq!(key(&mut router, 1, 16, None, true)["commit"], "ni");
    assert_eq!(key(&mut router, 1, 16, None, true)["commit"], json!(null));
    key(&mut router, 1, 16, None, false);
    key(&mut router, 1, 16, None, true);
    compose(&mut router, 1, "ni");
    let response = key(&mut router, 1, 9, None, false);
    let identity = response["identity"].clone();
    let expected = response["frame"]["candidates"]["items"][0]["text"].clone();
    assert_eq!(
        event(
            &mut router,
            1,
            json!({"Candidate": {"identity": identity, "index": 0}})
        )["commit"],
        expected
    );
    assert_eq!(
        event(
            &mut router,
            1,
            json!({"Candidate": {"identity": identity, "index": 0}})
        )["commit"],
        json!(null)
    );
    for (client_preedit, capability_changed, expected) in [
        (true, false, json!(null)),
        (false, false, json!("ni")),
        (false, true, json!(null)),
    ] {
        compose(&mut router, 1, "ni");
        assert_eq!(
            event(
                &mut router,
                1,
                json!({"Deactivate": {"focus_out": true, "client_preedit": client_preedit, "capability_changed": capability_changed}})
            )["commit"],
            expected
        );
    }
}
#[test]
fn focus_switches_preserve_context_text_but_reject_old_candidate_actions() {
    let mut router = router();
    let first = compose(&mut router, 1, "ni");
    caps(&mut router, 2, true, false, false);
    compose(&mut router, 2, "nihao");
    assert_eq!(
        event(
            &mut router,
            1,
            json!({"Candidate": {"identity": first["identity"], "index": 0}})
        )["commit"],
        json!(null)
    );
    assert_eq!(key(&mut router, 1, 13, None, false)["commit"], "ni");
    assert_eq!(key(&mut router, 2, 13, None, false)["commit"], "nihao");
}
