//! 标点全半角、表达式模式、问字模式。

use crate::support::*;

#[test]
fn chinese_punctuation_is_full_width_only_when_not_composing() {
    let mut router = router();
    // 没在组句：逗号转全角；数字后的点保持半角。
    let comma = KeyEvent::new(0xBC, Some(','), Default::default());
    assert_eq!(
        press(&mut router, comma),
        (
            KeyOutcome::Consumed,
            Some("，".to_owned()),
            Frame::default()
        )
    );
    press(&mut router, digit(3));
    let period = KeyEvent::new(0xBE, Some('.'), Default::default());
    assert_eq!(press(&mut router, period).0, KeyOutcome::Passthrough);
    assert_eq!(press(&mut router, period).1, Some("。".to_owned()));

    // 组句中：标点进英文直输段，不转。
    type_letters(&mut router, "ni");
    let (outcome, commit, frame) = press(&mut router, comma);
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert!(!frame.is_empty(), "组句应还在");

    // 状态条上关掉全角：原样交给应用。
    router.handle(ClientMessage::Commit { session: SESSION });
    router.handle_status_event(StatusEvent::TogglePunctuation);
    assert_eq!(press(&mut router, comma).0, KeyOutcome::Passthrough);
}

#[test]
fn expression_mode_takes_digits_and_operators() {
    let mut router = router();
    // v 开头进表达式模式：数字不选词、运算符进算式，Shift + 6 是 `^` 而不是删候选键。
    type_letters(&mut router, "v");
    press(&mut router, digit(1));
    press(&mut router, punct('+'));
    let (outcome, commit, frame) = press(&mut router, digit(2));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&frame), "v1+2");
    assert_eq!(candidate_texts(&frame), ["3", "1+2=3"]);
    press(&mut router, digit_with(6, SHIFT));
    let (_, _, frame) = press(&mut router, digit(2));
    assert_eq!(preedit(&frame), "v1+2^2");
    assert_eq!(candidate_texts(&frame), ["5", "1+2^2=5"]);
    // 空格上屏首选并清空。
    let (outcome, commit, frame) = press(&mut router, punct(' '));
    assert_eq!(
        (outcome, commit),
        (KeyOutcome::Consumed, Some("5".to_owned()))
    );
    assert!(preedit(&frame).is_empty());
}

#[test]
fn expression_mode_spells_chinese_numerals() {
    let mut router = router();
    type_letters(&mut router, "v");
    for n in [1, 2, 3] {
        press(&mut router, digit(n));
    }
    let (_, _, frame) = press(&mut router, punct('.'));
    assert_eq!(preedit(&frame), "v123.");
    let (_, _, frame) = press(&mut router, digit(5));
    assert_eq!(
        candidate_texts(&frame),
        [
            "一百二十三点五",
            "壹佰贰拾叁点伍",
            "一百二十三元五角",
            "壹佰贰拾叁元伍角"
        ]
    );
    router.handle(ClientMessage::Key {
        session: SESSION,
        event: KeyEvent::new(0x1B, None, Default::default()),
    });
    type_letters(&mut router, "v");
    for n in [1, 2, 3] {
        press(&mut router, digit(n));
    }
    let (_, _, frame) = press(&mut router, punct('+'));
    // `v123+` 算不出来就没有候选，回车上屏原文。
    assert!(candidate_texts(&frame).is_empty());
    let (_, _, frame) = press(&mut router, KeyEvent::new(0x08, None, Default::default()));
    assert_eq!(
        candidate_texts(&frame),
        [
            "一百二十三",
            "壹佰贰拾叁",
            "一百二十三元整",
            "壹佰贰拾叁元整"
        ]
    );
    press(&mut router, KeyEvent::new(0x28, None, Default::default()));
    let (_, commit, _) = press(&mut router, punct(' '));
    assert_eq!(commit.as_deref(), Some("壹佰贰拾叁"));
}

#[test]
fn expression_mode_other_punctuation_commits_then_applies() {
    let mut router = router();
    type_letters(&mut router, "v");
    press(&mut router, digit(1));
    press(&mut router, punct('+'));
    press(&mut router, digit(2));
    // 逗号不是算式的一部分：先把首选上屏，逗号按没在组句处理（中文模式转全角）。
    let (outcome, commit, frame) = press(&mut router, punct(','));
    assert_eq!(
        (outcome, commit),
        (KeyOutcome::Consumed, Some("3，".to_owned()))
    );
    assert!(preedit(&frame).is_empty());
}

#[test]
fn question_key_unicode_entry_takes_digits() {
    let mut router = router();
    type_letters(&mut router, "u");
    press(&mut router, digit(4));
    type_letters(&mut router, "e");
    press(
        &mut router,
        KeyEvent::new(0x30, Some('0'), Default::default()),
    );
    let (_, _, frame) = press(
        &mut router,
        KeyEvent::new(0x30, Some('0'), Default::default()),
    );
    assert_eq!(preedit(&frame), "u4e00");
    assert_eq!(candidate_texts(&frame), ["一"]);
    let (_, commit, _) = press(&mut router, punct(' '));
    assert_eq!(commit.as_deref(), Some("一"));
    // `u+1f600`：`+` 也进缓冲区。
    type_letters(&mut router, "u");
    press(&mut router, punct('+'));
    press(&mut router, digit(1));
    type_letters(&mut router, "f");
    press(&mut router, digit(6));
    press(
        &mut router,
        KeyEvent::new(0x30, Some('0'), Default::default()),
    );
    let (_, _, frame) = press(
        &mut router,
        KeyEvent::new(0x30, Some('0'), Default::default()),
    );
    assert_eq!(candidate_texts(&frame), ["😀"]);
}

#[test]
fn bare_question_mark_is_plain_punctuation_by_default() {
    let mut router = router();
    // 缺省 `?` 不进问字：中文模式直接出全角问号，英文模式半角。
    let (outcome, commit, frame) = press(&mut router, punct('?'));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert_eq!(commit.as_deref(), Some("？"));
    assert!(preedit(&frame).is_empty());
    let (outcome, commit, _) = press(&mut router, KeyEvent::new(0xBF, Some('?'), ENGLISH));
    assert_eq!((outcome, commit), (KeyOutcome::Passthrough, None));
}

#[test]
fn bare_question_mark_enters_question_mode_in_both_modes() {
    let mut router = router_asking();
    // 中文模式：`?` 进问字模式不上屏，后面的字母是问题。
    let (outcome, commit, frame) = press(&mut router, punct('?'));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&frame), "?");
    let (_, commit, frame) = type_letters(&mut router, "sangemu");
    assert_eq!(commit, None);
    assert!(preedit(&frame).starts_with('?'), "{}", preedit(&frame));
    press(&mut router, function_key(0x1B));
    // 英文模式也一样，Caps 送来的大写字母按小写收。
    let (outcome, commit, frame) = press(&mut router, KeyEvent::new(0xBF, Some('?'), CAPS));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&frame), "?");
    let (_, _, frame) = press(&mut router, letter_with('S', CAPS));
    assert_eq!(preedit(&frame), "?s");
}

#[test]
fn bare_question_mark_restores_when_followed_by_other_keys() {
    let mut router = router_asking();
    // 空格只是把这个 ? 上屏（中文模式全角），不多打空格。
    press(&mut router, punct('?'));
    let (outcome, commit, frame) = press(&mut router, punct(' '));
    assert_eq!(
        (outcome, commit),
        (KeyOutcome::Consumed, Some("？".to_owned()))
    );
    assert!(preedit(&frame).is_empty());
    // 回车同样只上屏问号，吞掉回车。
    press(&mut router, punct('?'));
    let (outcome, commit, _) = press(&mut router, function_key(0x0D));
    assert_eq!(
        (outcome, commit),
        (KeyOutcome::Consumed, Some("？".to_owned()))
    );
    // 其他字符：问号上屏后按没在组句处理（逗号转全角）。
    press(&mut router, punct('?'));
    let (_, commit, _) = press(&mut router, punct(','));
    assert_eq!(commit.as_deref(), Some("？，"));
    // 退格删掉它，什么都不上屏。
    press(&mut router, punct('?'));
    let (outcome, commit, frame) = press(&mut router, function_key(0x08));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert!(preedit(&frame).is_empty());
    // 英文模式还原成半角。
    press(&mut router, KeyEvent::new(0xBF, Some('?'), ENGLISH));
    let (_, commit, _) = press(&mut router, KeyEvent::new(0x20, Some(' '), ENGLISH));
    assert_eq!(commit.as_deref(), Some("?"));
}

#[test]
fn bare_question_mark_is_half_width_when_full_width_is_off() {
    let mut router = router_asking_with(RouterConfig {
        full_width: false,
        ..RouterConfig::default()
    });
    press(&mut router, punct('?'));
    let (_, commit, _) = press(&mut router, punct(' '));
    assert_eq!(commit.as_deref(), Some("?"));
}

#[test]
fn shuangpin_semicolon_stays_in_buffer_in_question_mode() {
    let mut router = router_asking_with(RouterConfig {
        shuangpin: Some(ShuangpinScheme::Microsoft),
        ..RouterConfig::default()
    });
    // 微软双拼的 `;` 是 ing 键：问字模式下末尾有落单声母时进缓冲区，而不是把候选上屏。
    press(&mut router, punct('?'));
    type_letters(&mut router, "x");
    let (outcome, commit, frame) = press(&mut router, punct(';'));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert!(!preedit(&frame).is_empty());
    assert_ne!(preedit(&frame), "?x");
}

#[test]
fn shuangpin_enters_modes_with_shifted_letters() {
    let mut router = router_with(RouterConfig {
        shuangpin: Some(ShuangpinScheme::Xiaohe),
        ..RouterConfig::default()
    });
    // Shift+V 进表达式：数字和运算符进缓冲区，空格上屏结果。
    let (outcome, commit, _) = press(&mut router, letter_with('V', SHIFT));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    press(&mut router, digit(1));
    press(&mut router, punct('+'));
    let (_, _, frame) = press(&mut router, digit(2));
    assert_eq!(preedit(&frame), "V1+2");
    assert_eq!(candidate_texts(&frame)[0], "3");
    let (_, commit, _) = press(&mut router, punct(' '));
    assert_eq!(commit.as_deref(), Some("3"));
    // Shift+U 进问字：码点本地答。
    press(&mut router, letter_with('U', SHIFT));
    press(&mut router, digit(4));
    type_letters(&mut router, "e");
    press(&mut router, digit(0));
    let (_, _, frame) = press(&mut router, digit(0));
    assert_eq!(candidate_texts(&frame), ["一"]);
    press(&mut router, function_key(0x1B));
    // 小写 v 仍是音节键；其他大写字母、全拼下的 Shift+V 照旧交给应用。
    let (_, _, frame) = type_letters(&mut router, "v");
    assert_eq!(preedit(&frame), "zh");
    press(&mut router, function_key(0x1B));
    assert_eq!(
        press(&mut router, letter_with('A', SHIFT)).0,
        KeyOutcome::Passthrough
    );
    let mut full = router_with(RouterConfig::default());
    let (outcome, _, frame) = press(&mut full, letter_with('V', SHIFT));
    assert_eq!(outcome, KeyOutcome::Passthrough);
    assert!(frame.is_empty());
}

#[test]
fn punctuation_toggle_is_remembered_per_mode() {
    let mut router = router_with(RouterConfig {
        status_enabled: true,
        ..RouterConfig::default()
    });
    let comma = KeyEvent::new(0xBC, Some(','), Default::default());
    let english_comma = KeyEvent::new(0xBC, Some(','), ENGLISH);
    // 中文模式下切成半角。
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: false,
    });
    router.handle_status_event(StatusEvent::TogglePunctuation);
    assert_eq!(press(&mut router, comma).0, KeyOutcome::Passthrough);
    // 英文模式缺省半角；点那一格切成全角，英文模式下真转。
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: true,
    });
    assert_eq!(press(&mut router, english_comma).0, KeyOutcome::Passthrough);
    router.handle_status_event(StatusEvent::TogglePunctuation);
    assert_eq!(press(&mut router, english_comma).1, Some("，".to_owned()));
    // 切回中文：还是中文自己记住的半角；再切回英文：还是英文记住的全角。
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: false,
    });
    assert_eq!(press(&mut router, comma).0, KeyOutcome::Passthrough);
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: true,
    });
    assert_eq!(press(&mut router, english_comma).1, Some("，".to_owned()));
    // 英文候选组词中敲标点：先把字母原样上屏，标点也按英文那份转。
    type_english(&mut router, "hello");
    let (_, commit, _) = press(&mut router, english_comma);
    assert_eq!(commit.as_deref(), Some("hello，"));
}
