//! 中英切换键（`[shortcut] switch_mode`）与初始模式（`[general] default_english`）。
use crate::support::{compose, key, key_with, router_with};
use qingjian_linux_server::RouterConfig;
use qingjian_platform::protocol::KeyModifiers;
use qingjian_platform::{SwitchKey, SwitchKeys};
use serde_json::json;

const SHIFT: u32 = 0x10;
const CONTROL: u32 = 0x11;

fn config(keys: &[SwitchKey]) -> RouterConfig {
    RouterConfig {
        switch_mode: keys
            .iter()
            .fold(SwitchKeys::NONE, |keys, key| keys.with(*key, true)),
        ..Default::default()
    }
}

/// 单击一个修饰键，返回松开那一下的结果。
fn tap(router: &mut qingjian_linux_server::Router, code: u32) -> serde_json::Value {
    key(router, 1, code, None, false);
    key(router, 1, code, None, true)
}

/// 中文模式下 `n` 进组句，英文模式（应用不在英文候选白名单）直接上屏。
fn english(router: &mut qingjian_linux_server::Router) -> bool {
    let english = compose(router, 1, "n")["commit"] == "n";
    key(router, 1, 0x1b, None, false);
    english
}

#[test]
fn empty_switch_keys_leave_shift_to_the_application() {
    let mut router = router_with(config(&[]));
    compose(&mut router, 1, "ni");
    assert_eq!(
        key(&mut router, 1, SHIFT, None, false)["outcome"],
        "Passthrough"
    );
    let release = key(&mut router, 1, SHIFT, None, true);
    assert_eq!(release["outcome"], "Passthrough");
    assert_eq!(release["commit"], json!(null));
    assert_eq!(key(&mut router, 1, 13, None, false)["commit"], "ni");
    assert!(!english(&mut router));
}

#[test]
fn control_tap_switches_only_when_chosen_and_alone() {
    let mut router = router_with(config(&[SwitchKey::Control]));
    assert_eq!(tap(&mut router, SHIFT)["outcome"], "Passthrough");
    assert!(!english(&mut router));
    let ctrl = KeyModifiers {
        ctrl: true,
        ..Default::default()
    };
    key(&mut router, 1, CONTROL, None, false);
    key_with(&mut router, 1, b'c' as u32, Some('c'), ctrl, false);
    assert_eq!(
        key_with(&mut router, 1, CONTROL, None, ctrl, true)["outcome"],
        "Passthrough"
    );
    assert!(!english(&mut router));
    compose(&mut router, 1, "ni");
    assert_eq!(
        key(&mut router, 1, CONTROL, None, false)["outcome"],
        "Passthrough"
    );
    let release = key_with(&mut router, 1, CONTROL, None, ctrl, true);
    assert_eq!(release["outcome"], "Consumed");
    assert_eq!(release["commit"], "ni");
    assert!(english(&mut router));
}

#[test]
fn ctrl_alt_space_switches_when_chosen() {
    let chord = KeyModifiers {
        ctrl: true,
        alt: true,
        ..Default::default()
    };
    let mut router = router_with(config(&[]));
    assert_eq!(
        key_with(&mut router, 1, 0x20, Some(' '), chord, false)["outcome"],
        "Passthrough"
    );
    assert!(!english(&mut router));
    let mut router = router_with(config(&[SwitchKey::CtrlAltSpace]));
    assert_eq!(
        key_with(&mut router, 1, 0x20, Some(' '), chord, false)["outcome"],
        "Consumed"
    );
    assert!(english(&mut router));
    key_with(&mut router, 1, 0x20, Some(' '), chord, false);
    assert!(!english(&mut router));
}

#[test]
fn default_english_starts_new_sessions_in_english() {
    let mut router = router_with(RouterConfig {
        default_english: true,
        ..config(&[SwitchKey::Shift])
    });
    assert!(english(&mut router));
    tap(&mut router, SHIFT);
    assert!(!english(&mut router));

    let mut router = router_with(RouterConfig {
        default_english: true,
        english_mode: false,
        ..Default::default()
    });
    assert!(!english(&mut router));
    assert_eq!(tap(&mut router, SHIFT)["outcome"], "Passthrough");
    assert!(!english(&mut router));
}

#[test]
fn the_switching_key_result_carries_the_new_mode() {
    // 插件靠 `english` 弹「中」/「英」气泡；没切换的按键必须是 null，免得误弹。
    let mut router = router_with(config(&[SwitchKey::Shift]));
    assert_eq!(tap(&mut router, SHIFT)["english"], json!(true));
    assert_eq!(tap(&mut router, SHIFT)["english"], json!(false));
    assert_eq!(
        key(&mut router, 1, b'n' as u32, Some('n'), false)["english"],
        json!(null)
    );
}
