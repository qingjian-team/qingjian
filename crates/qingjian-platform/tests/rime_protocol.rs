//! 原生按键和显示协议不能套用青简自己的字母键码或九候选分页。
use qingjian_core::{Candidate, CandidateKind, CandidateList, Query, RimeMenu};
use qingjian_platform::protocol::{Frame, KeyEvent, KeyModifiers};
use qingjian_platform::{Config, LayoutMode, PreeditMode, ThemeMode};

#[test]
fn raw_keysym_overrides_colliding_windows_virtual_key() {
    let mut event = KeyEvent::new(0x70, Some('p'), KeyModifiers::default());
    event.keysym = Some('p' as u32);
    assert_eq!(event.rime_key().0, 'p' as i32);
    event.keysym = None;
    assert_eq!(event.rime_key().0, 0xffbe);
}

#[test]
fn release_and_modifiers_reach_native_composer() {
    let mut event = KeyEvent::new(0xa1, None, KeyModifiers::default());
    event.release = true;
    event.modifiers.ctrl = true;
    event.modifiers.shift = true;
    assert_eq!(event.rime_key(), (0xffe2, (1 << 30) | (1 << 2) | 1));
    event.virtual_key = 0x50;
    event.character = Some('\u{10}');
    assert_eq!(event.rime_key().0, 'P' as i32);
    event.virtual_key = 0x61;
    assert_eq!(event.rime_key().0, 0xffb1);
}

#[test]
fn old_key_payload_and_default_config_keep_native_backend_disabled() {
    let event: KeyEvent = serde_json::from_str(r#"{"virtual_key":65,"character":"a","modifiers":{"ctrl":false,"shift":false,"alt":false,"win":false}}"#).unwrap();
    assert!(!event.release);
    assert_eq!(event.keysym, None);
    let config: Config = toml::from_str("").unwrap();
    assert!(config.rime.options().is_none());
    let configured: Config = toml::from_str(
        r#"[rime]
enabled = true
library = '/native/librime.so'
shared_data = '/data/rime-ice'
user_data = '/data/user'
schema = 'double_pinyin_flypy'
"#,
    )
    .unwrap();
    let options = configured.rime.options().unwrap();
    assert_eq!(options.schema, "double_pinyin_flypy");
    assert_eq!(options.shared_data.to_str(), Some("/data/rime-ice"));
}

#[test]
fn frame_preserves_native_page_size_highlight_and_unicode_caret() {
    let candidates = CandidateList {
        items: (0..12)
            .map(|index| Candidate {
                text: format!("候选{index}"),
                kind: CandidateKind::Chinese,
                syllables: Vec::new(),
                reading: None,
                translation: None,
                aux_code: None,
                rime: None,
            })
            .collect(),
    };
    let query = Query {
        text: "nihao".into(),
        cursor: 3,
        typed_display: Some("你好 ni".into()),
        candidates,
        rime_menu: Some(RimeMenu {
            page: 2,
            page_size: 12,
            last_page: false,
            highlighted: 11,
            cursor: 1,
        }),
        ..Query::default()
    };
    let frame = Frame::from_rime(
        query,
        PreeditMode::Both,
        LayoutMode::Vertical,
        ThemeMode::System,
    );
    assert_eq!(frame.candidates.items.len(), 12);
    assert_eq!(frame.highlight, 11);
    assert_eq!(frame.cursor, 1);
    assert_eq!(frame.page, 2);
    assert_eq!(frame.page_count, 4);
}

#[test]
fn macos_modifier_release_is_independent_of_the_opposite_key() {
    for (left, right, aggregate, left_bit, right_bit, left_sym, right_sym) in [
        (56, 60, 1 << 17, 0x02, 0x04, 0xffe1, 0xffe2),
        (59, 62, 1 << 18, 0x01, 0x2000, 0xffe3, 0xffe4),
        (58, 61, 1 << 19, 0x20, 0x40, 0xffe9, 0xffea),
        (55, 54, 1 << 20, 0x08, 0x10, 0xffeb, 0xffec),
    ] {
        let both = aggregate | left_bit | right_bit;
        for (code, sym) in [(left, left_sym), (right, right_sym)] {
            let event = KeyEvent::from_macos_rime(code, None, both, false, true).unwrap();
            assert!(!event.release);
            assert_eq!(event.rime_key().0, sym);
        }
        let right_up =
            KeyEvent::from_macos_rime(right, None, aggregate | left_bit, false, true).unwrap();
        let left_up =
            KeyEvent::from_macos_rime(left, None, aggregate | right_bit, false, true).unwrap();
        assert!(right_up.release && left_up.release);
        assert_eq!(right_up.rime_key().1 & (1 << 30), 1 << 30);
        assert_eq!(left_up.rime_key().1 & (1 << 30), 1 << 30);
    }
}

#[test]
fn macos_function_keypad_unicode_and_release_reach_rime() {
    for (code, symbol) in [
        (36, 0xff0d),
        (76, 0xff8d),
        (82, 0xffb0),
        (92, 0xffb9),
        (69, 0xffab),
        (105, 0xffca),
        (90, 0xffd1),
    ] {
        assert_eq!(
            KeyEvent::from_macos_rime(code, None, 0, false, false)
                .unwrap()
                .rime_key()
                .0,
            symbol
        );
    }
    let event = KeyEvent::from_macos_rime(0, Some('你'), 0, true, false).unwrap();
    assert_eq!(event.rime_key(), (0x0100_0000 | '你' as i32, 1 << 30));
    assert!(KeyEvent::from_macos_rime(0, None, 0, false, false).is_none());
}
