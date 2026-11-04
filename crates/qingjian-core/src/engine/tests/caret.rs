//! 点拼音行挪光标（#881）。

use super::*;

/// 点拼音行第 `index` 个字之后，回 (缓冲区光标, 新的拼音行)。
fn tap(engine: &mut Engine, index: usize) -> (usize, String) {
    let query = engine.query().unwrap();
    assert!(engine.move_cursor_to_marked(&query, index));
    let query = engine.query().unwrap();
    (query.cursor, query.marked_text())
}

#[test]
fn full_pinyin_tap_skips_the_shown_apostrophes() {
    let mut engine = engine();
    engine.set_input("nihaoma");
    assert_eq!(engine.query().unwrap().marked_text(), "ni'hao'ma");
    // ni'|hao：落在 h 前
    assert_eq!(tap(&mut engine, 3).0, 2);
    // ni|'hao 也是 h 前
    engine.move_cursor_end();
    assert_eq!(tap(&mut engine, 2).0, 2);
    // nih|ao：音节中间也能停
    engine.move_cursor_end();
    assert_eq!(tap(&mut engine, 4).0, 3);
    // 行首、行尾
    assert_eq!(tap(&mut engine, 0).0, 0);
    assert_eq!(tap(&mut engine, 99).0, 7);
}

#[test]
fn full_pinyin_tap_keeps_typed_apostrophes() {
    let mut engine = engine();
    engine.set_input("xi'an");
    assert_eq!(engine.query().unwrap().marked_text(), "xi'an");
    // xi'|an：越过敲的 `'`
    assert_eq!(tap(&mut engine, 3).0, 3);
}

#[test]
fn typing_after_a_tap_inserts_at_the_tapped_place() {
    let mut engine = engine();
    engine.set_input("nhao");
    let query = engine.query().unwrap();
    assert!(engine.move_cursor_to_marked(&query, 1));
    engine.push('i');
    assert_eq!(engine.composition().text(), "nihao");
}

#[test]
fn shuangpin_tap_snaps_to_the_nearest_syllable() {
    let mut engine = xiaohe();
    engine.set_input("kdfa");
    assert_eq!(engine.query().unwrap().marked_text(), "kai'fa");
    // ka|i'fa：后半个音节，落到 kai 后
    assert_eq!(tap(&mut engine, 2).0, 2);
    engine.move_cursor_end();
    // k|ai'fa：前半，落到开头
    assert_eq!(tap(&mut engine, 1).0, 0);
    engine.move_cursor_end();
    assert_eq!(tap(&mut engine, 4).0, 2);
    assert_eq!(tap(&mut engine, 6).0, 4);
}

#[test]
fn shuangpin_raw_preedit_maps_key_for_key() {
    let mut engine = xiaohe();
    engine.set_shuangpin_raw_preedit(true);
    engine.set_input("kdfa");
    assert_eq!(tap(&mut engine, 3).0, 3);
}
