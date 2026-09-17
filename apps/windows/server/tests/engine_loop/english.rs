//! 英文模式：候选与选词、Caps 直出、按应用关候选、中英切换时的缓冲。

use crate::support::*;

#[test]
fn english_mode_gives_candidates_and_space_picks_highlighted() {
    let mut router = router();
    let (outcome, commit, frame) = type_english(&mut router, "hel");
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&frame), "hel", "英文模式敲的字母原样显示");
    let texts = candidate_texts(&frame);
    assert!(
        texts.contains(&"hello") && texts.contains(&"help"),
        "候选应来自英文词表：{texts:?}"
    );
    // 空格与中文模式一样选高亮的词，词后接上空格（放行会让应用先插空格）。
    let first = frame.candidates.items[0].text.clone();
    let (outcome, commit, after) = press(&mut router, KeyEvent::new(0x20, Some(' '), ENGLISH));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert_eq!(commit, Some(format!("{first} ")));
    assert!(after.is_empty());

    // 回车仍把字母原样上屏：词表没有的写法靠它。
    type_english(&mut router, "hel");
    let (outcome, commit, _) = press(&mut router, function_key(0x0D));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("hel"))
    );
}

#[test]
fn english_digits_pick_candidates_or_join_the_word() {
    let mut router = router();
    // 有候选：数字选当前页第 N 个，与中文模式一样。
    let (_, _, frame) = type_english(&mut router, "hel");
    let second = frame.candidates.items[1].text.clone();
    let (outcome, commit, after) = press(&mut router, KeyEvent::new(0x32, Some('2'), ENGLISH));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, Some(second)));
    assert!(after.is_empty());

    // 没候选（词表没有的词）：数字是标识符的一部分，回车整段原样上屏。
    let (_, _, frame) = type_english(&mut router, "xq");
    assert!(
        frame.candidates.items.is_empty(),
        "样例词表里没有 xq 开头的词"
    );
    let (outcome, commit, frame) = press(&mut router, KeyEvent::new(0x31, Some('1'), ENGLISH));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&frame), "xq1");
    let (_, commit, _) = press(&mut router, function_key(0x0D));
    assert_eq!(commit.as_deref(), Some("xq1"));
}

#[test]
fn caps_lock_types_direct_uppercase_english_regardless_of_mode() {
    let mut router = router();
    let (outcome, commit, frame) = press(&mut router, letter_with('H', CAPS));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("H"))
    );
    assert!(frame.is_empty(), "Caps 直接上屏不出候选：{frame:?}");
    // 组句中 Caps 亮着敲字母：拼音先原样上屏，再接大写字母。
    type_letters(&mut router, "ni");
    let (_, commit, after) = press(&mut router, letter_with('A', CAPS));
    assert_eq!(commit.as_deref(), Some("niA"));
    assert!(after.is_empty());
}

#[test]
fn english_candidates_are_off_in_listed_apps_by_exe_name() {
    // VS Code 在缺省名单里（exe 名不区分大小写）：英文模式字母直插、不出候选。
    let mut router = router_in_app("code.exe");
    let (outcome, commit, frame) = press(&mut router, letter_with('h', ENGLISH));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("h"))
    );
    assert!(frame.is_empty(), "名单里的应用不该有候选：{frame:?}");
    let (outcome, commit, _) = press(&mut router, KeyEvent::new(0x20, Some(' '), ENGLISH));
    assert_eq!((outcome, commit), (KeyOutcome::Passthrough, None));
    // 中文模式不受名单影响。
    let (_, _, frame) = type_letters(&mut router, "ni");
    assert!(!frame.candidates.items.is_empty(), "拼音照常出候选");
}

#[test]
fn english_candidates_stay_on_in_other_apps() {
    let mut router = router_in_app("notepad.exe");
    let (outcome, commit, frame) = type_english(&mut router, "hel");
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert!(
        candidate_texts(&frame).contains(&"hello"),
        "不在名单里的应用照常给英文候选：{frame:?}"
    );
}

#[test]
fn app_list_is_looked_up_per_session() {
    // 两个应用同时在线：切会话时按各自的 exe 名判断。
    let mut router = router_in_app("Code.exe");
    let notepad = SessionId(2);
    router.handle(ClientMessage::OpenSession {
        session: notepad,
        app: Some("notepad.exe".to_owned()),
        protocol: PROTOCOL_VERSION,
    });
    let (_, _, frame) = key_result(router.handle(ClientMessage::Key {
        session: notepad,
        event: letter_with('h', ENGLISH),
    }));
    assert_eq!(preedit(&frame), "h", "记事本会话组词");
    // 切回编辑器会话：残留组句清掉，字母直插。
    let (outcome, commit, after) = press(&mut router, letter_with('e', ENGLISH));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("e"))
    );
    assert!(after.is_empty());
}

#[test]
fn english_tab_and_arrow_keys_pick_candidates() {
    let mut router = router();
    let (_, _, frame) = type_english(&mut router, "hel");
    let first = frame.candidates.items[0].text.clone();
    // Tab 选高亮的词。
    let (outcome, commit, _) = press(&mut router, KeyEvent::new(0x09, None, ENGLISH));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some(first.as_str()))
    );

    // 方向键移到第二个之后，空格选的是它，再接上空格。
    let (_, _, frame) = type_english(&mut router, "hel");
    let second = frame.candidates.items[1].text.clone();
    let (outcome, _, _) = press(&mut router, KeyEvent::new(0x28, None, ENGLISH));
    assert_eq!(outcome, KeyOutcome::Consumed);
    let (_, commit, after) = press(&mut router, KeyEvent::new(0x20, Some(' '), ENGLISH));
    assert_eq!(commit, Some(format!("{second} ")));
    assert!(after.is_empty());
}

#[test]
fn english_without_candidates_is_passthrough_with_shift_case() {
    let mut router = router_with(RouterConfig {
        english_candidates: false,
        ..RouterConfig::default()
    });
    // 字母由我们插入，大小写按 Shift；不组句。
    let (outcome, commit, frame) = press(&mut router, letter_with('h', ENGLISH));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("h"))
    );
    assert!(frame.is_empty());
    let shifted = KeyModifiers {
        shift: true,
        ..ENGLISH
    };
    let (_, commit, _) = press(&mut router, letter_with('H', shifted));
    assert_eq!(commit.as_deref(), Some("H"));
    // 其他键交给应用。
    let (outcome, commit, _) = press(&mut router, KeyEvent::new(0x20, Some(' '), ENGLISH));
    assert_eq!((outcome, commit), (KeyOutcome::Passthrough, None));
}

#[test]
fn switching_to_chinese_mid_word_flushes_english_letters() {
    let mut router = router();
    type_english(&mut router, "hel");
    // 切回中文模式再敲字母：之前的英文字母原样上屏，新字母从头当拼音。
    let (outcome, commit, frame) = press(&mut router, letter('l'));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("hel"))
    );
    assert_eq!(preedit(&frame), "l");
}

#[test]
fn english_digit_without_a_slot_joins_the_word() {
    let mut router = router();
    let (_, _, frame) = type_english(&mut router, "hello");
    let shown = frame.candidates.items.len();
    assert!((1..9).contains(&shown), "hello 的候选应不满 9 个：{shown}");
    let (outcome, commit, frame) = press(&mut router, digit_with(9, ENGLISH));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&frame), "hello9");
}
