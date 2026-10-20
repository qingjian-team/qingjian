//! 中英文单击切换的事件序列回归。

use super::*;

#[test]
fn tap_switches_only_on_release_on_either_side() {
    let mut mode = ModeSwitch::default();
    mode.sync_caps(false);
    for (key, side) in [(56, 2), (60, 4)] {
        let before = mode.english();
        assert!(!mode.flags(key, SHIFT | side));
        assert_eq!(mode.english(), before);
        assert!(mode.flags(key, 0));
        assert_ne!(mode.english(), before);
    }
}

#[test]
fn typing_and_other_modifiers_cancel_tap() {
    let mut mode = ModeSwitch::default();
    mode.sync_caps(false);
    mode.flags(56, SHIFT | 2);
    mode.key_down(0, SHIFT | 2, false);
    assert!(!mode.flags(56, 0));
    mode.flags(56, SHIFT | 2);
    mode.flags(55, SHIFT | COMMAND | 2 | 8);
    mode.flags(55, SHIFT | 2);
    assert!(!mode.flags(56, 0));
    assert!(!mode.english());
}

#[test]
fn overlapping_shifts_and_focus_loss_do_not_switch() {
    let mut mode = ModeSwitch::default();
    mode.sync_caps(false);
    mode.flags(56, SHIFT | 2);
    mode.flags(60, SHIFT | 6);
    assert!(!mode.flags(56, SHIFT | 4));
    assert!(!mode.flags(60, 0));
    mode.flags(56, SHIFT | 2);
    mode.cancel();
    assert!(!mode.flags(56, 0));
    // 在另一个应用按下 Shift 后切入，松开不能当单击。
    assert!(!mode.flags(60, 0));
    assert!(!mode.english());
}

#[test]
fn caps_changes_set_mode_but_do_not_override_later_taps() {
    let mut mode = ModeSwitch::default();
    assert!(mode.sync_caps(true));
    mode.flags(56, CAPS | SHIFT | 2);
    assert!(mode.flags(56, CAPS));
    assert!(!mode.english());
    assert!(!mode.sync_caps(true));
    assert!(!mode.english());
    assert!(!mode.sync_caps(false));
    assert!(mode.sync_caps(true));
}

#[test]
fn caps_change_config_change_and_fn_cancel_pending_tap() {
    let mut mode = ModeSwitch::default();
    mode.sync_caps(false);
    mode.flags(56, SHIFT | 2);
    assert!(mode.flags(57, CAPS | SHIFT | 2));
    assert!(!mode.flags(56, CAPS));
    assert!(mode.english());
    mode.sync_caps(false);
    mode.flags(56, SHIFT | 2);
    mode.configure(true, SwitchKeys::NONE);
    assert!(!mode.flags(56, 0));
    mode.configure(true, SwitchKeys::default());
    assert!(!mode.flags(56, SHIFT | FUNCTION | 2));
    assert!(!mode.flags(56, FUNCTION));
    assert!(!mode.english());
}

#[test]
fn mode_change_requires_cleanup_until_successful_drain() {
    let mut mode = ModeSwitch::default();
    mode.sync_caps(false);
    assert!(!mode.needs_finish());
    mode.flags(56, SHIFT | 2);
    mode.flags(56, 0);
    assert!(mode.needs_finish());
    // Host 借用失败时不确认清理；普通按键、失焦也不能丢掉待清理状态。
    mode.cancel();
    mode.key_down(0, 0, false);
    assert!(mode.needs_finish());
    mode.did_finish();
    assert!(!mode.needs_finish());
    mode.configure(false, SwitchKeys::default());
    assert!(mode.needs_finish());
}

#[test]
fn configuration_and_ctrl_option_space_are_respected() {
    let mut mode = ModeSwitch::default();
    mode.sync_caps(false);
    mode.configure(true, SwitchKeys::NONE);
    mode.flags(56, SHIFT | 2);
    assert!(!mode.flags(56, 0));
    let keys = SwitchKeys {
        shift: false,
        control: true,
        ctrl_alt_space: true,
    };
    mode.configure(true, keys);
    mode.flags(62, CONTROL | 0x2000);
    assert!(mode.flags(62, 0));
    assert!(mode.key_down(49, CONTROL | OPTION, false));
    assert!(!mode.english());
    assert!(mode.key_down(49, CONTROL | OPTION, true));
    assert!(!mode.english());
    assert!(!mode.key_down(49, CONTROL | OPTION | COMMAND, false));
    mode.configure(false, keys);
    mode.sync_caps(true);
    assert!(!mode.english());
    assert!(!mode.key_down(49, CONTROL | OPTION, false));
}

#[test]
fn public_and_mixed_device_flags_switch_on_either_side() {
    for (key, side) in [(56, 2), (60, 4), (59, 1), (62, 0x2000)] {
        let group = if key == 56 || key == 60 {
            SHIFT
        } else {
            CONTROL
        };
        for down_device in [0, side] {
            for up_device in [0, side] {
                let mut mode = ModeSwitch::default();
                mode.configure(
                    true,
                    SwitchKeys::default().with(qingjian_platform::SwitchKey::Control, true),
                );
                mode.sync_caps(false);
                assert!(!mode.flags(key, group | down_device));
                assert_eq!(mode.pending(), Some(key));
                assert!(mode.flags(key, up_device));
                assert!(mode.english());
            }
        }
    }
}

#[test]
fn overlapping_sides_never_rearm_before_group_release() {
    for (first, second, first_side, second_side, group) in [
        (56, 60, 2, 4, SHIFT),
        (60, 56, 4, 2, SHIFT),
        (59, 62, 1, 0x2000, CONTROL),
        (62, 59, 0x2000, 1, CONTROL),
    ] {
        for release_first in [false, true] {
            // 独立变化每一条事件的设备位，覆盖只有公共位与混合来源的事件。
            for mask in 0..16 {
                let mut mode = ModeSwitch::default();
                mode.configure(
                    true,
                    SwitchKeys::default().with(qingjian_platform::SwitchKey::Control, true),
                );
                mode.sync_caps(false);
                let (released, remaining, remaining_side) = if release_first {
                    (first, second, second_side)
                } else {
                    (second, first, first_side)
                };
                let sequence = [
                    (first, group, first_side),
                    (second, group, first_side | second_side),
                    (released, group, remaining_side),
                    (remaining, 0, 0),
                ];
                for (index, (key, common, devices)) in sequence.into_iter().enumerate() {
                    let flags = common | if mask & (1 << index) != 0 { devices } else { 0 };
                    assert!(!mode.flags(key, flags));
                    assert!(!mode.english());
                }
                // 整组释放后，下一次正常单击仍可用。
                mode.flags(first, group);
                assert!(mode.flags(first, 0));
            }
        }
    }
}

#[test]
fn interrupted_public_shift_taps_do_not_switch() {
    let mut mode = ModeSwitch::default();
    mode.sync_caps(false);
    mode.flags(56, SHIFT);
    mode.key_down(0, SHIFT, false);
    assert!(!mode.flags(56, 0));
    for (key, flags) in [(55, COMMAND), (58, OPTION), (59, CONTROL), (63, FUNCTION)] {
        mode.flags(56, SHIFT);
        mode.flags(key, SHIFT | flags);
        mode.flags(key, SHIFT);
        assert!(!mode.flags(56, 0));
    }
    // 鼠标／失焦取消，且失焦期间可能已经松开 Shift。
    mode.flags(56, SHIFT);
    mode.cancel_with_flags(SHIFT);
    assert!(!mode.flags(56, 0));
    mode.flags(56, SHIFT);
    mode.cancel_with_flags(0);
    mode.flags(56, SHIFT);
    assert!(mode.flags(56, 0));
}

#[test]
fn native_flags_changed_events_preserve_public_only_tap() {
    use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType};
    use objc2_foundation::{NSPoint, NSString};

    let mut mode = ModeSwitch::default();
    mode.sync_caps(false);
    let empty = NSString::from_str("");
    for key in [56, 60] {
        let before = mode.english();
        for (flags, changed) in [
            (NSEventModifierFlags::Shift, false),
            (NSEventModifierFlags::empty(), true),
        ] {
            let event = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
                NSEventType::FlagsChanged,
                NSPoint::ZERO,
                flags,
                0.0,
                0,
                None,
                &empty,
                &empty,
                false,
                key,
            ).expect("AppKit 必须能创建修饰键事件");
            assert_eq!(event.r#type(), NSEventType::FlagsChanged);
            assert_eq!(event.modifierFlags().bits() & 0xffff, 0);
            assert_eq!(
                mode.flags(event.keyCode(), event.modifierFlags().bits()),
                changed
            );
        }
        assert_ne!(mode.english(), before);
    }
}
