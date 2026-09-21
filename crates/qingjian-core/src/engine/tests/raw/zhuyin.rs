//! 注音按单元映射光标，保留轻声重排、一声隐藏与尾串的既有输出。
use super::assert_raw;
use crate::engine::tests::engine;

#[test]
fn raw_preedit_zhuyin_cursor_follows_output_character_boundaries() {
    for (keys, text, positions) in [
        ("", "", vec![0]),
        ("su", "ㄋㄧ", vec![0, 3, 6]),
        ("1j4", "ㄅㄨˋ", vec![0, 3, 6, 8]),
        ("1j41j4", "ㄅㄨˋㄅㄨˋ", vec![0, 3, 6, 8, 11, 14, 16]),
        ("su ", "ㄋㄧ", vec![0, 3, 6, 6]),
        ("7su", "ㄋㄧ˙", vec![0, 3, 6, 8]),
        ("su'1j4", "ㄋㄧ'ㄅㄨˋ", vec![0, 3, 6, 7, 10, 13, 15]),
        ("#su", "ㄋㄧ#", vec![0, 7, 3, 7]),
        ("s#u?", "ㄋㄧ#?", vec![0, 3, 7, 6, 8]),
        ("s#'u?", "ㄋ'ㄧ#?", vec![0, 3, 8, 4, 7, 9]),
        ("4su", "ㄋㄧ4", vec![0, 7, 3, 7]),
    ] {
        for (position, cursor) in positions.into_iter().enumerate() {
            let mut engine = engine();
            engine.set_zhuyin_mode(true);
            engine.set_input(keys);
            engine.move_cursor_home();
            for _ in 0..position {
                engine.move_cursor_right();
            }
            assert_raw(&mut engine, text, cursor);
        }
    }
}

#[test]
fn raw_preedit_zhuyin_middle_edit_and_english_fallback() {
    let mut engine = engine();
    engine.set_zhuyin_mode(true);
    engine.set_input("1j41j4");
    for _ in 0..3 {
        engine.move_cursor_left();
    }
    engine.backspace();
    engine.push('6');
    assert_raw(&mut engine, "ㄅㄨˊㄅㄨˋ", 8);
    engine.set_english_mode(true);
    engine.set_input("su3");
    engine.move_cursor_left();
    assert_raw(&mut engine, "su3", 2);
}
