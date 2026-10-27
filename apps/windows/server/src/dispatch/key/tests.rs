//! Tab 与分页的三端约定；直接注入整句补全状态，不接云服务。
use crate::dispatch::{Router, RouterConfig};
use qingjian_core::candidate::arrows::Focus;
use qingjian_core::{CustomPhrase, Engine};
use qingjian_dictionary::{Dictionary, WordList};
use qingjian_platform::protocol::{
    ClientMessage, Frame, KeyEvent, KeyModifiers, KeyOutcome, PROTOCOL_VERSION, ServerMessage,
    SessionId,
};
use qingjian_platform::{AppsConfig, HighlightKeys, LayoutMode};

pub(super) fn router(size: usize) -> Router {
    let mut engine = Engine::new(Dictionary::parse("你\tni\t100\n").unwrap()).with_english(
        WordList::parse("hello\thello\t100\nhelp\thelp\t90\nheld\theld\t80\n").unwrap(),
    );
    engine
        .set_custom_phrases(
            (1..=9)
                .map(|n| CustomPhrase::new("qq", &format!("第{n}项")))
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
pub(super) fn key(
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
pub(super) fn compose(router: &mut Router, text: &str, modifiers: KeyModifiers) {
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
/// #250：候选窗上滚轮翻页、点翻页箭头，和翻页键一样到头就停。
#[test]
fn wheel_and_page_arrows_turn_pages_like_page_keys() {
    use crate::dispatch::CandidateMouse;

    let mut router = router(4);
    let normal = KeyModifiers::default();
    router.handle_candidate_mouse(CandidateMouse::Page(1));
    assert_eq!(router.self_drawn_frame().page, 0, "没在打字时滚轮不动");
    compose(&mut router, "qq", normal);
    router.handle_candidate_mouse(CandidateMouse::Page(1));
    assert_eq!(router.self_drawn_frame().page, 1);
    router.handle_candidate_mouse(CandidateMouse::Page(5));
    assert_eq!(router.self_drawn_frame().page, 2);
    router.handle_candidate_mouse(CandidateMouse::Click(qingjian_render::HitTarget::Page(-1)));
    let frame = router.self_drawn_frame();
    assert_eq!((frame.page, frame.highlight), (1, 0));
    let expected = frame.candidates.items[0].text.clone();
    assert_eq!(
        key(&mut router, b'1' as u32, Some('1'), normal).1,
        Some(expected),
        "滚过去之后数字键选的是这一页"
    );
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

/// 横排矩阵（`[general] horizontal_grid` + 横排）：↓ 展开成固定视口（一行一页、空位是占位候选）、
/// ← → 在候选之间按阅读顺序移动、数字键选高亮所在那一行、Esc 第一下收回单行。
#[test]
fn horizontal_grid_expands_and_navigates() {
    const DOWN: u32 = 0x28;
    const LEFT: u32 = 0x25;
    const RIGHT: u32 = 0x27;
    let mut router = router(2);
    let normal = KeyModifiers::default();
    router.config.layout = LayoutMode::Horizontal;
    router.config.horizontal_grid = true;
    compose(&mut router, "qq", normal);
    // ↓：从单行展开，高亮下移一行（第二页第一个）；5 页全进视口，最后一格是空位占位
    let frame = key(&mut router, DOWN, None, normal).2;
    assert_eq!((frame.columns, frame.highlight, frame.page), (2, 2, 1));
    assert_eq!(frame.candidates.items.len(), 10);
    assert_eq!(frame.candidates.items[9].text, "");
    assert_eq!(frame.column_ems.len(), 2);
    // 再往下一行；← → 按阅读顺序移一格
    let frame = key(&mut router, DOWN, None, normal).2;
    assert_eq!((frame.highlight, frame.page), (4, 2));
    assert_eq!(key(&mut router, LEFT, None, normal).2.highlight, 3);
    assert_eq!(key(&mut router, RIGHT, None, normal).2.highlight, 4);
    // 数字键选高亮所在那一行（高亮 4 在第三页，`2` 选第 6 个候选）
    assert_eq!(
        key(&mut router, 0x32, Some('2'), normal).1.as_deref(),
        Some("第6项")
    );
    // Esc 第一下只收回单行（高亮留在原候选上、帧回到页内下标），第二下才清空
    compose(&mut router, "qq", normal);
    key(&mut router, DOWN, None, normal);
    let frame = key(&mut router, 0x1B, None, normal).2;
    assert_eq!((frame.columns, frame.highlight, frame.page), (0, 0, 1));
    assert_eq!(frame.candidates.items.len(), 2);
    assert_eq!(
        key(&mut router, 0x1B, None, normal)
            .2
            .candidates
            .items
            .len(),
        0
    );
}

/// 矩阵展开着时翻页键一次翻一屏（六行），到头夹住。
#[test]
fn horizontal_grid_page_keys_turn_screens() {
    let mut router = router(2);
    let normal = KeyModifiers::default();
    let shift = KeyModifiers {
        shift: true,
        ..normal
    };
    router.config.layout = LayoutMode::Horizontal;
    router.config.horizontal_grid = true;
    compose(&mut router, "qq", normal);
    key(&mut router, 0x28, None, normal);
    // Tab 翻一屏：从第二页跳过六行到最后一页
    let frame = key(&mut router, 9, None, normal).2;
    assert_eq!((frame.columns, frame.highlight, frame.page), (2, 8, 4));
    // Shift + Tab 翻回一屏：夹回首行
    let frame = key(&mut router, 9, None, shift).2;
    assert_eq!((frame.highlight, frame.page), (0, 0));
}

/// 开关关着时横排按键与以前一样：↑ ↓ 逐个移高亮（不展开），← → 移拼音光标，Alt + ← 归应用。
#[test]
fn horizontal_grid_off_keeps_old_keys() {
    let mut router = router(2);
    let normal = KeyModifiers::default();
    router.config.layout = LayoutMode::Horizontal;
    compose(&mut router, "qq", normal);
    let frame = key(&mut router, 0x28, None, normal).2;
    assert_eq!((frame.columns, frame.highlight), (0, 1));
    // ← 移拼音光标：重查候选，高亮归零
    let frame = key(&mut router, 0x25, None, normal).2;
    assert_eq!((frame.columns, frame.highlight, frame.cursor), (0, 0, 1));
    // Alt + ←：没开矩阵，带修饰键的方向键归应用
    assert_eq!(
        key(
            &mut router,
            0x25,
            None,
            KeyModifiers {
                alt: true,
                ..normal
            }
        )
        .0,
        KeyOutcome::Passthrough
    );
}

/// 矩阵里 ← → 与高亮键选 ← / → 时同一套规则：在候选之间按阅读顺序移动，
/// ← 顶在第一个候选上直接交还拼音光标（移动光标重查候选，矩阵跟着收回）。
#[test]
fn horizontal_grid_left_at_first_hands_back_to_the_pinyin_cursor() {
    const LEFT: u32 = 0x25;
    let mut router = router(2);
    let normal = KeyModifiers::default();
    router.config.layout = LayoutMode::Horizontal;
    router.config.horizontal_grid = true;
    compose(&mut router, "qq", normal);
    key(&mut router, 0x28, None, normal); // ↓ 展开成矩阵，高亮到第二行
    key(&mut router, 0x26, None, normal); // ↑ 回到第一行最左
    // ← 顶在第一个候选上：交还拼音光标（去改已输入的拼音）。帧里的光标在显示串上——
    // 单声母 `q` 后自动补的 `'` 让它从 3 退到 1；移动光标重查候选，矩阵收回单行
    let frame = key(&mut router, LEFT, None, normal).2;
    assert_eq!((frame.columns, frame.highlight, frame.cursor), (0, 0, 1));
}

/// 交还拼音光标后 ← / → 都归光标：移到最左还能按 → 移回来，光标回到末尾后 ← / → 才重新归候选。
#[test]
fn horizontal_grid_cursor_edit_arrows_move_back_to_the_end() {
    const LEFT: u32 = 0x25;
    const RIGHT: u32 = 0x27;
    let mut router = router(2);
    let normal = KeyModifiers::default();
    router.config.layout = LayoutMode::Horizontal;
    router.config.horizontal_grid = true;
    compose(&mut router, "qq", normal);
    // ← 顶在第一个候选上：交还拼音光标（末尾 3 → 1），高亮不动
    let frame = key(&mut router, LEFT, None, normal).2;
    assert_eq!((frame.columns, frame.highlight, frame.cursor), (0, 0, 1));
    // 再 ←：光标继续左移（到串首 0），高亮不动
    let frame = key(&mut router, LEFT, None, normal).2;
    assert_eq!((frame.highlight, frame.cursor), (0, 0));
    // → 把光标移回来：还在光标模式里，候选高亮一直没动
    let frame = key(&mut router, RIGHT, None, normal).2;
    assert_eq!((frame.highlight, frame.cursor), (0, 1));
    let frame = key(&mut router, RIGHT, None, normal).2;
    assert_eq!((frame.highlight, frame.cursor), (0, 3));
    // 光标回到末尾后：→ 才重新归候选
    assert_eq!(key(&mut router, RIGHT, None, normal).2.highlight, 1);
}

/// 第一排再按 ↑：高亮回到第一个候选（不是毫无动作）。
#[test]
fn horizontal_grid_up_at_the_first_row_returns_to_the_first_candidate() {
    const DOWN: u32 = 0x28;
    const UP: u32 = 0x26;
    let mut router = router(2);
    let normal = KeyModifiers::default();
    router.config.layout = LayoutMode::Horizontal;
    router.config.horizontal_grid = true;
    compose(&mut router, "qq", normal);
    key(&mut router, DOWN, None, normal); // ↓ 展开，高亮到第二行（下标 2）
    let frame = key(&mut router, 0x27, None, normal).2; // → 高亮到下标 3
    assert_eq!(frame.highlight, 3);
    let frame = key(&mut router, UP, None, normal).2; // ↑ 回第一排（同列，下标 1）
    assert_eq!(frame.highlight, 1);
    // 第一排再按 ↑：回到第一个候选
    let frame = key(&mut router, UP, None, normal).2;
    assert_eq!((frame.columns, frame.highlight, frame.page), (2, 0, 0));
    // 已在第一个候选上再按 ↑：不动（帧照旧）
    let frame = key(&mut router, UP, None, normal).2;
    assert_eq!((frame.columns, frame.highlight), (2, 0));
}

/// 横排、方向键分两区（高亮移动键选 ← / →）的路由器；`grid` 是开没开「横排时展开成多行」。
fn two_area_router(grid: bool) -> Router {
    let mut router = router(2);
    router.config.layout = LayoutMode::Horizontal;
    router.config.highlight_keys = HighlightKeys::LeftRight;
    router.config.horizontal_grid = grid;
    router
}

/// 候选里 ↑ 进拼音行（候选不画高亮），← → 只移拼音光标（到末尾不再溢出成选词），↓ 回候选，再 ↓ 才展开。
/// 帧里的光标在显示串上——单声母 `q` 后自动补的 `'` 让末尾是 3。
#[test]
fn two_area_arrows_switch_between_pinyin_and_candidates() {
    const LEFT: u32 = 0x25;
    const UP: u32 = 0x26;
    const RIGHT: u32 = 0x27;
    const DOWN: u32 = 0x28;
    let mut router = two_area_router(true);
    let normal = KeyModifiers::default();
    compose(&mut router, "qq", normal);
    let frame = key(&mut router, UP, None, normal).2;
    assert_eq!(
        (frame.columns, frame.highlight, frame.cursor),
        (0, usize::MAX, 3)
    );
    assert_eq!(key(&mut router, LEFT, None, normal).2.cursor, 1);
    assert_eq!(key(&mut router, RIGHT, None, normal).2.cursor, 3);
    let frame = key(&mut router, RIGHT, None, normal).2;
    assert_eq!((frame.highlight, frame.cursor), (usize::MAX, 3));
    assert_eq!(key(&mut router, DOWN, None, normal).2.highlight, 0);
    assert_eq!(key(&mut router, RIGHT, None, normal).2.highlight, 1);
    assert_eq!(key(&mut router, DOWN, None, normal).2.columns, 2);
}

/// 矩阵里 ↑ 先往上换行，到第一行再 ↑ 回拼音行，矩阵不收起（收起用 Esc）；候选里的 ↑ 不开符号面板，拼音行里再按 ↑ 才开。
#[test]
fn two_area_up_climbs_rows_then_returns_to_pinyin_and_opens_the_panel() {
    const UP: u32 = 0x26;
    const DOWN: u32 = 0x28;
    let mut router = two_area_router(true);
    let normal = KeyModifiers::default();
    compose(&mut router, "qq", normal);
    key(&mut router, DOWN, None, normal); // ↓ 展开，高亮到第二行（下标 2）
    assert_eq!(key(&mut router, UP, None, normal).2.highlight, 0);
    let frame = key(&mut router, UP, None, normal).2;
    assert_eq!((frame.columns, frame.symbol_panel), (2, false));
    assert_eq!(router.focus, Focus::Pinyin);

    let mut router = two_area_router(false);
    compose(&mut router, "qq", normal);
    assert!(!key(&mut router, UP, None, normal).2.symbol_panel);
    assert!(key(&mut router, UP, None, normal).2.symbol_panel);
    // 没开展开：候选里 ↓ 不动
    let mut router = two_area_router(false);
    compose(&mut router, "qq", normal);
    assert_eq!(key(&mut router, DOWN, None, normal).2.columns, 0);
}

/// 自动空格：上屏汉字后接着打数字补空格；退格删掉那个字后不知道上文，不补。
#[test]
fn auto_space_follows_commits_and_backspace() {
    let mut router = router(5);
    router.engine.set_auto_space(true);
    let normal = KeyModifiers::default();
    compose(&mut router, "ni", normal);
    assert_eq!(
        key(&mut router, 0x20, Some(' '), normal).1.as_deref(),
        Some("你")
    );
    let (outcome, commit, _) = key(&mut router, 0x33, Some('3'), normal);
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some(" 3"))
    );
    // 数字后面再打数字不补
    assert_eq!(
        key(&mut router, 0x34, Some('4'), normal).0,
        KeyOutcome::Passthrough
    );

    let mut router = self::router(5);
    router.engine.set_auto_space(true);
    compose(&mut router, "ni", normal);
    key(&mut router, 0x20, Some(' '), normal);
    assert_eq!(
        key(&mut router, 0x08, None, normal).0,
        KeyOutcome::Passthrough
    );
    assert_eq!(
        key(&mut router, 0x33, Some('3'), normal).0,
        KeyOutcome::Passthrough
    );
}
