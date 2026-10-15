//! Tab 与分页的三端约定；直接注入整句补全状态，不接云服务。
use crate::dispatch::{Router, RouterConfig};
use qingjian_core::{CustomPhrase, Engine};
use qingjian_dictionary::{Dictionary, WordList};
use qingjian_platform::AppsConfig;
use qingjian_platform::protocol::{
    ClientMessage, Frame, KeyEvent, KeyModifiers, KeyOutcome, PROTOCOL_VERSION, ServerMessage,
    SessionId,
};

fn router(size: usize) -> Router {
    let mut engine = Engine::new(Dictionary::parse("你\tni\t100\n").unwrap()).with_english(
        WordList::parse("hello\thello\t100\nhelp\thelp\t90\nheld\theld\t80\n").unwrap(),
    );
    engine
        .set_custom_phrases(
            (1..=9)
                .map(|position| CustomPhrase {
                    code: "qq".into(),
                    text: format!("第{position}项"),
                    position,
                    enabled: true,
                })
                .collect(),
        )
        .unwrap();
    let mut router = Router::new(
        engine,
        RouterConfig {
            page_size: size,
            apps: AppsConfig::with_english_candidates_on(&["*"]),
            ..Default::default()
        },
    );
    router.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    router
}
fn key(
    router: &mut Router,
    code: u32,
    character: Option<char>,
    modifiers: KeyModifiers,
) -> (KeyOutcome, Option<String>, Frame) {
    match router
        .handle(ClientMessage::Key {
            session: SessionId(1),
            event: KeyEvent::new(code, character, modifiers),
        })
        .unwrap()
    {
        ServerMessage::KeyResult {
            outcome,
            commit,
            frame,
            ..
        } => (outcome, commit, frame),
        _ => panic!("key result"),
    }
}
fn compose(router: &mut Router, text: &str, modifiers: KeyModifiers) {
    for c in text.chars() {
        key(router, c as u32, Some(c), modifiers);
    }
}
#[test]
fn tab_and_backtab_page_boundaries_and_current_page_selection() {
    for size in [1, 4, 5, 9] {
        let mut router = router(size);
        let normal = KeyModifiers::default();
        let shift = KeyModifiers {
            shift: true,
            ..normal
        };
        assert_eq!(key(&mut router, 9, None, normal).0, KeyOutcome::Passthrough);
        assert_eq!(key(&mut router, 9, None, shift).0, KeyOutcome::Passthrough);
        compose(&mut router, "qq", normal);
        assert_eq!(key(&mut router, 9, None, shift).2.page, 0);
        let last = 9_usize.div_ceil(size) - 1;
        for page in 1..=last {
            let result = key(&mut router, 9, None, normal);
            assert_eq!(result.0, KeyOutcome::Consumed);
            assert_eq!((result.2.page, result.2.highlight), (page, 0));
        }
        let frame = key(&mut router, 9, None, normal).2;
        assert_eq!(frame.page, last);
        let expected = frame.candidates.items[0].text.clone();
        assert_eq!(
            key(&mut router, b'1' as u32, Some('1'), normal).1,
            Some(expected)
        );
        compose(&mut router, "qq", normal);
        key(&mut router, 0x22, None, normal);
        assert_eq!(key(&mut router, 9, None, shift).2.page, 0);
        assert_eq!(key(&mut router, 0x21, None, normal).2.page, 0);
    }
}
#[test]
fn shift_tab_precedes_prediction_and_english_commit() {
    let mut router = router(1);
    let normal = KeyModifiers::default();
    compose(&mut router, "qq", normal);
    router.sentence = Some("可控补全".into());
    let result = key(
        &mut router,
        9,
        None,
        KeyModifiers {
            shift: true,
            ..normal
        },
    );
    assert_eq!(result.1, None);
    assert_eq!(router.sentence.as_deref(), Some("可控补全"));
    assert_eq!(
        key(&mut router, 9, None, normal).1.as_deref(),
        Some("可控补全")
    );
    let english = KeyModifiers {
        english_mode: true,
        ..normal
    };
    compose(&mut router, "hel", english);
    key(&mut router, 0x22, None, english);
    let previous = key(
        &mut router,
        9,
        None,
        KeyModifiers {
            shift: true,
            ..english
        },
    );
    assert_eq!(previous.1, None);
    assert_eq!(previous.2.page, 0);
    let expected = previous.2.candidates.items[previous.2.highlight]
        .text
        .clone();
    assert_eq!(key(&mut router, 9, None, english).1, Some(expected));
}
#[test]
fn tab_with_raw_input_and_no_candidates_is_consumed_without_commit() {
    let mut router = router(5);
    compose(&mut router, "zzzz", KeyModifiers::default());
    let result = key(&mut router, 9, None, KeyModifiers::default());
    assert_eq!(result.0, KeyOutcome::Consumed);
    assert_eq!(result.1, None);
    assert_eq!(result.2.page, 0);
}

#[test]
fn all_extra_pairs_page_chinese_candidates_without_committing() {
    let mut baseline = router(1);
    let mut router = router(1);
    router.config.extra_page_keys = vec!["[]".into(), ",.".into(), "-=".into()]
        .try_into()
        .unwrap();
    let normal = KeyModifiers::default();
    compose(&mut router, "qq", normal);
    for pair in [('[', ']'), (',', '.'), ('-', '=')] {
        let next = key(&mut router, pair.1 as u32, Some(pair.1), normal);
        assert_eq!(
            (next.0, next.1, next.2.page),
            (KeyOutcome::Consumed, None, 1)
        );
        let previous = key(&mut router, pair.0 as u32, Some(pair.0), normal);
        assert_eq!(
            (previous.0, previous.1, previous.2.page),
            (KeyOutcome::Consumed, None, 0)
        );
    }
    // 清掉额外键后，逗号恢复为原有输入行为。
    router.config.extra_page_keys = Default::default();
    compose(&mut baseline, "qq", normal);
    let expected = key(&mut baseline, 0xBC, Some(','), normal);
    let actual = key(&mut router, 0xBC, Some(','), normal);
    assert_eq!((actual.0, actual.1), (expected.0, expected.1));
    assert_eq!(router.engine.composition(), baseline.engine.composition());
}

/// 组句中会转全角的标点：先把高亮候选上屏再补标点（`ni,` 出「你，」）；
/// 半角标点模式下候选照样上屏、标点按半角补。`,` `.` 配成翻页键时翻页优先，不上屏。
#[test]
fn punctuation_commits_highlighted_candidate() {
    let mut router = router(5);
    router.config.punct_commits = true;
    let normal = KeyModifiers::default();
    compose(&mut router, "ni", normal);
    let result = key(&mut router, 0xBC, Some(','), normal);
    assert_eq!(result.0, KeyOutcome::Consumed);
    assert_eq!(result.1.as_deref(), Some("你，"));
    router.config.full_width = false;
    compose(&mut router, "ni", normal);
    let result = key(&mut router, 0xBC, Some(','), normal);
    assert_eq!(result.1.as_deref(), Some("你,"));
    router.config.page_keys = (',', '.');
    compose(&mut router, "ni", normal);
    let result = key(&mut router, 0xBC, Some(','), normal);
    assert_eq!(result.0, KeyOutcome::Consumed);
    assert_eq!(result.1, None);
    assert_eq!(result.2.page, 0);
}
/// 不会转全角的符号（`-`）仍进英文直输段；`'` 是隔音符，进缓冲区不触发上屏，随后的标点照常上屏候选。
#[test]
fn unconvertible_symbols_do_not_commit_candidates() {
    let mut router = router(5);
    router.config.punct_commits = true;
    let normal = KeyModifiers::default();
    compose(&mut router, "ni", normal);
    let result = key(&mut router, 0xBD, Some('-'), normal);
    assert_eq!(result.1, None);
    let result = key(&mut router, 0x20, Some(' '), normal);
    assert_eq!(result.1.as_deref(), Some("ni- "));
    compose(&mut router, "ni", normal);
    let result = key(&mut router, 0xDE, Some('\''), normal);
    assert_eq!(result.1, None);
    let result = key(&mut router, 0xBC, Some(','), normal);
    assert_eq!(result.1.as_deref(), Some("你，"));
}
/// 直通了数字再组句，`Punctuation` 的「数字后的点保持半角」状态要跟着刷新：`3` + `ni` + `.` 出「你。」不出「你.」。
#[test]
fn digit_then_composition_resets_decimal_point_state() {
    let mut router = router(5);
    router.config.punct_commits = true;
    let normal = KeyModifiers::default();
    key(&mut router, 0x33, Some('3'), normal);
    compose(&mut router, "ni", normal);
    let result = key(&mut router, 0xBE, Some('.'), normal);
    assert_eq!(result.1.as_deref(), Some("你。"));
}
/// `[general] punct_commits` 关着（缺省）：组句中的标点进英文直输段，不上屏候选；打开后上屏候选再补标点。
#[test]
fn punct_commits_off_keeps_punctuation_in_raw_segment() {
    let mut router = router(5);
    let normal = KeyModifiers::default();
    router.config.punct_commits = false;
    compose(&mut router, "ni", normal);
    assert_eq!(key(&mut router, 0xBC, Some(','), normal).1, None);
    assert_eq!(
        key(&mut router, 0x20, Some(' '), normal).1.as_deref(),
        Some("ni, ")
    );
    router.config.punct_commits = true;
    compose(&mut router, "ni", normal);
    assert_eq!(
        key(&mut router, 0xBC, Some(','), normal).1.as_deref(),
        Some("你，")
    );
}

/// 英文候选里 `-` `=` 作翻页键：`-` 翻得回上一页；第一页没得翻时 `-` 仍是单词里的连字符。
#[test]
fn english_minus_pages_back_and_stays_a_hyphen_on_the_first_page() {
    let mut router = router(1);
    router.config.page_keys = ('-', '=');
    let english = KeyModifiers {
        english_mode: true,
        ..Default::default()
    };
    compose(&mut router, "hel", english);
    assert_eq!(key(&mut router, 0xBB, Some('='), english).2.page, 1);
    let back = key(&mut router, 0xBD, Some('-'), english);
    assert_eq!(
        (back.0, back.1, back.2.page),
        (KeyOutcome::Consumed, None, 0)
    );
    assert_eq!(router.engine.composition().text(), "hel");
    key(&mut router, 0xBD, Some('-'), english);
    assert_eq!(router.engine.composition().text(), "hel-");
}

/// 中文组句里 `-` `=` 翻不动时落回原字符进直输段；`[` `]` 翻不动照旧吃键。
#[test]
fn chinese_minus_equals_fall_back_to_characters_at_page_edges() {
    let normal = KeyModifiers::default();
    let mut r = router(4);
    r.config.page_keys = ('-', '=');
    compose(&mut r, "qq", normal);
    key(&mut r, 0xBD, Some('-'), normal);
    assert_eq!(r.engine.composition().text(), "qq-");

    let mut r = router(4);
    r.config.page_keys = ('-', '=');
    compose(&mut r, "qq", normal);
    for page in 1..=2 {
        assert_eq!(key(&mut r, 0xBB, Some('='), normal).2.page, page);
    }
    assert_eq!(key(&mut r, 0xBD, Some('-'), normal).2.page, 1);
    key(&mut r, 0xBB, Some('='), normal);
    key(&mut r, 0xBB, Some('='), normal);
    assert_eq!(r.engine.composition().text(), "qq=");

    let mut r = router(4);
    compose(&mut r, "qq", normal);
    let result = key(&mut r, 0xDB, Some('['), normal);
    assert_eq!(
        (result.0, result.1, result.2.page),
        (KeyOutcome::Consumed, None, 0)
    );
    assert_eq!(r.engine.composition().text(), "qq");
}
