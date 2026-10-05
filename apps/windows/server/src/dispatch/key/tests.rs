//! Tab 与分页的三端约定；直接注入整句补全状态，不接云服务。
use crate::dispatch::{Router, RouterConfig};
use qingjian_core::{CustomPhrase, Engine};
use qingjian_dictionary::{Dictionary, WordList};
use qingjian_platform::LayoutMode;
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
/// 组句中会转全角的标点：先把高亮候选上屏再补标点（`ni,` 出「你，」）；
/// 半角标点模式下候选照样上屏、标点按半角补。`,` `.` 配成翻页键时翻页优先，不上屏。
#[test]
fn punctuation_commits_highlighted_candidate() {
    let mut router = router(5);
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
    let normal = KeyModifiers::default();
    key(&mut router, 0x33, Some('3'), normal);
    compose(&mut router, "ni", normal);
    let result = key(&mut router, 0xBE, Some('.'), normal);
    assert_eq!(result.1.as_deref(), Some("你。"));
}
/// 关掉 `[general] punct_commits`：组句中的标点回到老行为——进英文直输段，不再上屏候选；开回来恢复。
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

/// 横排矩阵（`[general] horizontal_grid` + 横排）：↓ 展开成固定视口（一行一页、空位是占位候选）、
/// ← → 在候选之间按阅读顺序移动、数字键选高亮所在那一行、Esc 第一下收回单行；
/// Alt + ← → 移拼音光标（移动光标重查候选，矩阵跟着收回，与 macOS 壳一致）。
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

/// 开着矩阵时 Alt + ← → 移拼音光标（矩阵把 ← → 占了，光标得有候补键位）。
#[test]
fn horizontal_grid_alt_arrows_move_pinyin_cursor() {
    let mut router = router(2);
    let normal = KeyModifiers::default();
    router.config.layout = LayoutMode::Horizontal;
    router.config.horizontal_grid = true;
    compose(&mut router, "qq", normal);
    key(&mut router, 0x28, None, normal);
    // Alt + ←：光标退一个音节；移动光标重查候选，矩阵收回单行
    let alt = KeyModifiers {
        alt: true,
        ..normal
    };
    let frame = key(&mut router, 0x25, None, alt).2;
    assert_eq!((frame.columns, frame.cursor), (0, 1));
    // Alt + →：跳回末尾。帧里的光标在显示串上——单声母 `q` 后自动补的 `'` 让它比敲的长
    let frame = key(&mut router, 0x27, None, alt).2;
    assert_eq!(frame.cursor, 3);
}

/// 照微信输入法的逻辑，← 顶在第一个候选上逐级退出：展开着先收回单行（高亮留在原处），
/// 单行里再按一下才交还拼音光标；单行里 ← 先逐个移高亮，顶到头才轮到光标。
#[test]
fn horizontal_grid_left_at_first_backs_out() {
    const LEFT: u32 = 0x25;
    const RIGHT: u32 = 0x27;
    let mut router = router(2);
    let normal = KeyModifiers::default();
    router.config.layout = LayoutMode::Horizontal;
    router.config.horizontal_grid = true;
    compose(&mut router, "qq", normal);
    key(&mut router, 0x28, None, normal); // ↓ 展开成矩阵，高亮到第二行
    key(&mut router, 0x26, None, normal); // ↑ 回到第一行最左
    // ← 顶在第一个候选上：收回单行，高亮留在原候选（帧回到页内下标）
    let frame = key(&mut router, LEFT, None, normal).2;
    assert_eq!((frame.columns, frame.highlight, frame.page), (0, 0, 0));
    assert_eq!(frame.candidates.items.len(), 2);
    // 单行里 → 仍先移高亮、← 走回来；只有再顶到第一个候选才轮到拼音光标
    assert_eq!(key(&mut router, RIGHT, None, normal).2.highlight, 1);
    assert_eq!(key(&mut router, LEFT, None, normal).2.highlight, 0);
    // ← 顶在第一个候选上：交还拼音光标（去改已输入的拼音）。帧里的光标在显示串上——
    // 单声母 `q` 后自动补的 `'` 让它从 3 退到 1
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
