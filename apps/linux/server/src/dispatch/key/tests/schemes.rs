//! 双拼模式入口与注音符号、声调、提交键边界。
use super::support::{compose, key, router};
use qingjian_core::{ModeKeys, ShuangpinScheme};
use qingjian_platform::protocol::{KeyModifiers, KeyOutcome};

#[test]
fn shuangpin_shift_mode_keys_respect_custom_bindings_and_english() {
    let mut router = router(5);
    let normal = KeyModifiers::default();
    router.engine.set_shuangpin(Some(ShuangpinScheme::Xiaohe));
    compose(&mut router, "V1+2", normal);
    assert!(router.engine.expression_mode());
    assert_eq!(
        key(&mut router, 32, Some(' '), normal).1.as_deref(),
        Some("3")
    );
    compose(&mut router, "U4e00", normal);
    assert!(router.engine.question_mode());
    assert_eq!(
        key(&mut router, 32, Some(' '), normal).1.as_deref(),
        Some("一")
    );
    router.engine.set_mode_keys(ModeKeys {
        expression: 'i',
        question: 'v',
        question_mark: false,
    });
    compose(&mut router, "I1+2", normal);
    assert!(router.engine.expression_mode());
    key(&mut router, 27, None, normal);
    let english = KeyModifiers {
        english_mode: true,
        ..normal
    };
    compose(&mut router, "I", english);
    assert!(router.engine.english_mode());
    assert_eq!(key(&mut router, 13, None, english).1.as_deref(), Some("I"));
    key(&mut router, 27, None, english);
    router.engine.set_shuangpin(None);
    assert_eq!(
        key(&mut router, b'I' as u32, Some('I'), normal).0,
        KeyOutcome::Passthrough
    );
}
#[test]
fn zhuyin_symbol_keys_and_tones_are_input_and_enter_selects() {
    let mut router = router(5);
    router.engine.set_zhuyin_mode(true);
    let normal = KeyModifiers::default();
    for c in "0123456789-;,./".chars() {
        let result = key(&mut router, c as u32, Some(c), normal);
        assert_eq!(result.0, KeyOutcome::Consumed, "{c}");
        assert_eq!(router.engine.composition().text(), c.to_string());
        key(&mut router, 27, None, normal);
    }
    // ㄋㄧˇ 对应你；声调前空格补一声，再空格上屏。
    compose(&mut router, "su3", normal);
    let result = key(&mut router, 13, None, normal);
    assert_eq!(result.1.as_deref(), Some("你"));
    compose(&mut router, "su3", normal);
    assert_eq!(
        key(
            &mut router,
            13,
            None,
            KeyModifiers {
                shift: true,
                ..normal
            }
        )
        .1
        .as_deref(),
        Some("ㄋㄧˇ")
    );
    compose(&mut router, "su", normal);
    let tone = key(&mut router, 32, Some(' '), normal);
    assert!(tone.1.is_none());
    assert_eq!(
        key(&mut router, 32, Some(' '), normal).1.as_deref(),
        Some("你")
    );
    compose(&mut router, "su3", normal);
    key(&mut router, 0x28, None, normal);
    let expected = router.current_frame();
    let text = &expected.candidates.items[expected.highlight].text;
    assert_eq!(key(&mut router, 13, None, normal).1.as_ref(), Some(text));
}
