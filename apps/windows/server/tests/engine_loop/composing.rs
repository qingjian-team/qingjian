//! 中文组句：出候选、选词、上屏、翻页键、直输段、失焦与学习落盘。

use crate::support::*;

#[test]
fn typing_pinyin_shows_candidates() {
    let mut router = router();
    let (outcome, commit, frame) = type_letters(&mut router, "nihao");

    assert_eq!(outcome, KeyOutcome::Consumed);
    assert_eq!(commit, None);
    assert_eq!(preedit(&frame), "ni'hao");
    let texts: Vec<&str> = frame
        .candidates
        .items
        .iter()
        .map(|c| c.text.as_str())
        .collect();
    assert!(
        texts.contains(&"你好"),
        "候选里应有「你好」，实际：{texts:?}"
    );
}

#[test]
fn selecting_by_digit_commits_and_clears() {
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let position = frame
        .candidates
        .items
        .iter()
        .position(|c| c.text == "你好")
        .expect("「你好」在候选页内");
    let (outcome, commit, after) = key_result(router.handle(ClientMessage::Key {
        session: SESSION,
        event: digit(position as u32 + 1),
    }));

    assert_eq!(outcome, KeyOutcome::Consumed);
    assert_eq!(commit.as_deref(), Some("你好"));
    assert!(
        after.is_empty(),
        "上屏后应收起候选，实际 preedit={:?}",
        preedit(&after)
    );
}

#[test]
fn space_commits_first_candidate() {
    let mut router = router();
    type_letters(&mut router, "ni");
    let space = KeyEvent::new(0x20, Some(' '), Default::default());
    let (outcome, commit, after) = key_result(router.handle(ClientMessage::Key {
        session: SESSION,
        event: space,
    }));

    assert_eq!(outcome, KeyOutcome::Consumed);
    assert_eq!(commit.as_deref(), Some("你"), "「ni」首选应是「你」");
    assert!(after.is_empty());
}

#[test]
fn backspace_shrinks_preedit() {
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    assert_eq!(preedit(&frame), "ni'hao");
    let back = KeyEvent::new(0x08, None, Default::default());
    let (outcome, _, after) = key_result(router.handle(ClientMessage::Key {
        session: SESSION,
        event: back,
    }));

    assert_eq!(outcome, KeyOutcome::Consumed);
    assert_eq!(preedit(&after), "ni'ha");
}

#[test]
fn non_letter_without_composing_passes_through() {
    let mut router = router();
    let space = KeyEvent::new(0x20, Some(' '), Default::default());
    let (outcome, commit, frame) = key_result(router.handle(ClientMessage::Key {
        session: SESSION,
        event: space,
    }));

    assert_eq!(outcome, KeyOutcome::Passthrough);
    assert_eq!(commit, None);
    assert!(frame.is_empty());
}

#[test]
fn focus_leave_commits_raw_pinyin() {
    let mut router = router();
    type_letters(&mut router, "nihao");
    let committed = router.handle(ClientMessage::Commit { session: SESSION });
    assert_eq!(
        committed,
        Some(ServerMessage::Committed {
            session: SESSION,
            text: Some("nihao".to_owned()),
        })
    );
    let (_, _, frame) = type_letters(&mut router, "ni");
    assert_eq!(preedit(&frame), "ni");
    // 没在组句时 Commit 不交东西。
    router.handle(ClientMessage::Key {
        session: SESSION,
        event: KeyEvent::new(0x1B, None, Default::default()),
    });
    assert_eq!(
        router.handle(ClientMessage::Commit { session: SESSION }),
        Some(ServerMessage::Committed {
            session: SESSION,
            text: None,
        })
    );
}

#[test]
fn commit_from_other_session_does_not_take_buffer() {
    let mut router = router();
    type_letters(&mut router, "ni");
    let other = SessionId(2);
    router.handle(ClientMessage::OpenSession {
        session: other,
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    // 别的会话拿不到这个会话的拼音，但残留组句一并清掉。
    assert_eq!(
        router.handle(ClientMessage::Commit { session: other }),
        Some(ServerMessage::Committed {
            session: other,
            text: None,
        })
    );
    let space = KeyEvent::new(0x20, Some(' '), Default::default());
    let (outcome, _, _) = key_result(router.handle(ClientMessage::Key {
        session: SESSION,
        event: space,
    }));
    assert_eq!(outcome, KeyOutcome::Passthrough);
}

#[test]
fn page_keys_follow_config() {
    // 每页 1 条保证多页；翻页键改成 `,` `.`。
    let mut router = router_with(RouterConfig {
        page_size: 1,
        page_keys: (',', '.'),
        ..RouterConfig::default()
    });
    let (_, _, frame) = type_letters(&mut router, "ni");
    assert!(frame.page_count > 1, "样例词库里 ni 应不止一个候选");
    assert_eq!(frame.page, 0);

    let key = |router: &mut Router, c| {
        key_result(router.handle(ClientMessage::Key {
            session: SESSION,
            event: punct(c),
        }))
    };
    let (outcome, commit, frame) = key(&mut router, '.');
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(frame.page, 1, "`.` 应翻到下一页");
    let (_, _, frame) = key(&mut router, ',');
    assert_eq!(frame.page, 0, "`,` 应翻回上一页");
    // 缺省的 `]` 此时不再翻页，进直输段。
    let (_, _, frame) = key(&mut router, ']');
    assert_eq!(frame.page, 0);
    assert!(
        preedit(&frame).contains(']'),
        "`]` 应进直输段：{}",
        preedit(&frame)
    );
}

#[test]
fn minus_equals_page_keys_preserve_expression_input() {
    let mut router = router_with(RouterConfig {
        page_size: 1,
        page_keys: ('-', '='),
        ..RouterConfig::default()
    });
    let (_, _, frame) = type_letters(&mut router, "ni");
    assert!(frame.page_count > 1);
    let (outcome, commit, frame) = press(&mut router, punct('='));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(frame.page, 1);
    assert_eq!(preedit(&frame), "ni");
    let (_, _, frame) = press(&mut router, punct('-'));
    assert_eq!(frame.page, 0);
    assert_eq!(preedit(&frame), "ni");
    press(&mut router, KeyEvent::new(0x1B, None, Default::default()));

    type_letters(&mut router, "v");
    for c in "2-1=".chars() {
        let (outcome, commit, _) = press(&mut router, punct(c));
        assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    }
    let (outcome, commit, _) = press(&mut router, punct(' '));
    assert_eq!(
        (outcome, commit),
        (KeyOutcome::Consumed, Some("2-1=1".to_owned()))
    );
}

#[test]
fn shift_uppercase_while_composing_commits_raw_first() {
    let mut router = router();
    type_letters(&mut router, "ni");
    // 中文模式按住 Shift 打大写字母：拼音原样上屏，字母跟在后面一起插。
    let shifted = KeyModifiers {
        shift: true,
        ..KeyModifiers::default()
    };
    let (outcome, commit, frame) = press(&mut router, letter_with('A', shifted));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("niA"))
    );
    assert!(frame.is_empty());
    // 没在组句时大写字母交给应用。
    let (outcome, commit, _) = press(&mut router, letter_with('A', shifted));
    assert_eq!((outcome, commit), (KeyOutcome::Passthrough, None));
}

#[test]
fn learning_data_persists_to_user_dir() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let user_dir =
        std::env::temp_dir().join(format!("qingjian-windows-learning-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&user_dir);
    std::fs::create_dir_all(&user_dir).unwrap();
    let engine = assembly::assemble(&AssemblySpec {
        glossary: Some((
            Language::English,
            root.join("assets/sample/glossary-en.tsv"),
        )),
        user_dir: Some(user_dir.clone()),
        ..AssemblySpec::new(root.join("assets/sample/dict.tsv"))
    })
    .unwrap();
    let mut router = Router::new(engine, RouterConfig::default());
    router.handle(ClientMessage::OpenSession {
        session: SESSION,
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let position = frame
        .candidates
        .items
        .iter()
        .position(|c| c.text == "你好")
        .unwrap();
    router.handle(ClientMessage::Key {
        session: SESSION,
        event: digit(position as u32 + 1),
    });
    // 关会话时落盘。
    router.handle(ClientMessage::CloseSession { session: SESSION });

    let user = std::fs::read_to_string(user_dir.join("user.tsv")).expect("user.tsv 应已写出");
    assert!(user.contains("你好"), "user.tsv 里应记了「你好」：{user}");
    assert!(user_dir.join("usage.tsv").is_file(), "usage.tsv 应已写出");
    let _ = std::fs::remove_dir_all(&user_dir);
}

#[test]
fn raw_segment_takes_digits_and_keeps_the_space() {
    // `-` 进英文直输段之后数字是内容不是选词键，空格整段原样上屏并保留空格（#28 排查时发现 `gpt-6` 丢了 6）。
    let mut router = router();
    let mut frame = Frame::default();
    for c in "gpt-6".chars() {
        let (outcome, commit, next) = press(&mut router, letter(c));
        assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
        frame = next;
    }
    assert_eq!(preedit(&frame), "gpt-6");
    let (outcome, commit, after) = press(&mut router, letter(' '));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("gpt-6 "))
    );
    assert!(after.is_empty());
}

#[test]
fn digit_without_a_slot_joins_the_buffer() {
    // 这一页没有第 9 格：数字是内容（`gpt9`），不再被静默吞掉；成了直输段之后空格整段上屏。
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "gpt");
    let shown = frame.candidates.items.len();
    assert!(
        (1..9).contains(&shown),
        "样例词库下 gpt 的候选应不满 9 个：{shown}"
    );
    let (outcome, commit, frame) = press(&mut router, digit(9));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&frame), "gpt9");
    let (_, commit, after) = press(&mut router, letter(' '));
    assert_eq!(commit.as_deref(), Some("gpt9 "));
    assert!(after.is_empty());
    // 有这一格照常选词。
    let (_, _, frame) = type_letters(&mut router, "ni");
    let first = frame.candidates.items[0].text.clone();
    let (_, commit, _) = press(&mut router, digit(1));
    assert_eq!(commit, Some(first));
}
