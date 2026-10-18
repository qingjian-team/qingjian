//! 组句中的方向键：`[shortcut] highlight_keys` 决定高亮由 `↑` / `↓` 还是 `←` / `→` 移动。

use crate::support::*;

const LEFT: u32 = 0x25;
const UP: u32 = 0x26;
const RIGHT: u32 = 0x27;
const DOWN: u32 = 0x28;
const HOME: u32 = 0x24;

fn leftright() -> Router {
    router_with(RouterConfig {
        highlight_keys: HighlightKeys::LeftRight,
        ..RouterConfig::default()
    })
}

#[test]
fn default_keeps_up_down_on_the_highlight_and_left_right_on_the_caret() {
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    assert!(
        frame.candidates.items.len() > 1,
        "样例词库 nihao 要不止一个候选才试得出高亮"
    );
    let caret_end = frame.cursor;

    let (outcome, _, frame) = press(&mut router, function_key(DOWN));
    assert_eq!((outcome, frame.highlight), (KeyOutcome::Consumed, 1));
    let (_, _, frame) = press(&mut router, function_key(UP));
    assert_eq!(frame.highlight, 0);

    // ← / → 只走拼音光标，高亮留在原处
    let (outcome, _, frame) = press(&mut router, function_key(LEFT));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(
        frame.cursor < caret_end,
        "缺省下 ← 该走拼音光标：{} → {}",
        caret_end,
        frame.cursor
    );
    assert_eq!(frame.highlight, 0);
}

#[test]
fn leftright_moves_the_caret_first_and_then_the_highlight() {
    let mut router = leftright();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    assert!(frame.candidates.items.len() > 1);
    let caret_end = frame.cursor;

    // 拼音光标已在末位：这一下溢出成选词，光标不动
    let (outcome, _, frame) = press(&mut router, function_key(RIGHT));
    assert_eq!(
        (outcome, frame.highlight, frame.cursor),
        (KeyOutcome::Consumed, 1, caret_end)
    );

    // ← 先把高亮退回首位，再一下才回拼音光标
    let (_, _, frame) = press(&mut router, function_key(LEFT));
    assert_eq!((frame.highlight, frame.cursor), (0, caret_end));
    let (_, _, frame) = press(&mut router, function_key(LEFT));
    assert!(frame.cursor < caret_end, "高亮在首位时 ← 该回拼音光标");
    assert_eq!(frame.highlight, 0);
}

#[test]
fn leftright_right_walks_the_caret_back_before_selecting() {
    let mut router = leftright();
    type_letters(&mut router, "nihao");
    let (_, _, mid) = press(&mut router, function_key(LEFT));
    assert_eq!(mid.highlight, 0, "光标不在末位，这一下不该动高亮");

    // 光标还没回到末位：→ 走光标，不是选词
    let (_, _, frame) = press(&mut router, function_key(RIGHT));
    assert!(frame.cursor > mid.cursor);
    assert_eq!(frame.highlight, 0);
}

#[test]
fn leftright_swallows_up_and_down() {
    let mut router = leftright();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    for key in [DOWN, UP] {
        let (outcome, commit, frame) = press(&mut router, function_key(key));
        assert_eq!(
            (outcome, commit, frame.highlight),
            (KeyOutcome::Consumed, None, 0)
        );
    }
    assert_eq!(preedit(&frame), "ni'hao", "吃掉但组句还在");
}

#[test]
fn leftright_eats_left_when_both_ends_are_reached() {
    let mut router = leftright();
    type_letters(&mut router, "nihao");
    press(&mut router, function_key(HOME));
    let (outcome, _, frame) = press(&mut router, function_key(LEFT));
    assert_eq!(
        (outcome, frame.cursor, frame.highlight),
        (KeyOutcome::Consumed, 0, 0),
        "高亮与拼音光标都在头也要吃掉，漏给应用会挪插入符"
    );
}

#[test]
fn arrows_pass_through_when_not_composing() {
    let mut router = leftright();
    for key in [LEFT, RIGHT, UP, DOWN] {
        let (outcome, _, _) = press(&mut router, function_key(key));
        assert_eq!(outcome, KeyOutcome::Passthrough);
    }
}
