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

fn nine_key() -> Engine {
    let mut engine = engine();
    engine.set_nine_key(true);
    engine
}

#[test]
fn nine_key_tap_maps_letter_for_key() {
    // 64426 读成两个音节（测试词库里是 mi'gao）：拼音行一个字母对一个键，显示的 `'` 不占键
    let mut engine = nine_key();
    engine.set_input("64426");
    assert_eq!(engine.query().unwrap().marked_text().len(), 6);
    let (cursor, marked) = tap(&mut engine, 3);
    assert_eq!(cursor, 2);
    // 光标后的键也读成拼音，不露出数字
    assert!(!marked.chars().any(|c| c.is_ascii_digit()), "{marked}");
    assert_eq!(marked.chars().filter(|c| *c != '\'').count(), 5);
    // 接着打的键插在光标处
    engine.push('4');
    assert_eq!(engine.composition().text(), "644426");
}

#[test]
fn nine_key_tap_counts_pinned_letters() {
    let mut engine = nine_key();
    engine.set_input("64426");
    assert!(engine.pin_nine_key("ni"));
    assert_eq!(engine.composition().text(), "ni'426");
    // ni'h|ao：越过选定时接的 `'`，停在 4 后
    assert_eq!(tap(&mut engine, 4).0, 4);
}

#[test]
fn nine_key_pin_before_the_cursor_keeps_the_rest() {
    let mut engine = nine_key();
    engine.set_input("64426");
    tap(&mut engine, 2);
    assert!(engine.pin_nine_key("ni"));
    assert_eq!(engine.composition().text(), "ni'426");
    assert_eq!(engine.composition().cursor(), 2);
}

#[test]
fn nine_key_tap_keeps_the_reading_before_the_cursor() {
    // 64 单独查时 米 比 你 常用、426 第一种写法是 gao；整段读成 ni'hao，点在 ni 后两段都照旧读
    let words = "米\tmi\t9000\n你\tni\t5000\n你好\tni hao\t8000\n";
    let mut engine = Engine::new(Dictionary::parse(words).unwrap());
    engine.set_nine_key(true);
    engine.set_input("64426");
    assert_eq!(engine.query().unwrap().marked_text(), "ni'hao");
    let (cursor, marked) = tap(&mut engine, 2);
    assert_eq!(cursor, 2);
    assert_eq!(marked, "ni'hao");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "你");
}

#[test]
fn nine_key_drag_keeps_the_first_reading() {
    // 沿拼音行一格格挪：挪到半个音节里拼音行会换读法，再挪回来仍是最早的 ni'hao
    let words = "米	mi	9000
你	ni	5000
你好	ni hao	8000
";
    let mut engine = Engine::new(Dictionary::parse(words).unwrap());
    engine.set_nine_key(true);
    engine.set_input("64426");
    for index in [5, 4, 1, 3] {
        tap(&mut engine, index);
    }
    let (cursor, marked) = tap(&mut engine, 2);
    assert_eq!(cursor, 2);
    assert_eq!(marked, "ni'hao");
}

#[test]
fn nine_key_backspace_after_the_cursor_unpins_first() {
    // 选了 ni，点在 ni 后按退格：先退回成数字、光标停在它们后面，后面的键也照旧读成拼音；
    // 单独一个 6 时 哦 最常用
    let words = "哦\to\t9500\n米\tmi\t9000\n你\tni\t5000\n你好\tni hao\t8000\n";
    let mut engine = Engine::new(Dictionary::parse(words).unwrap());
    engine.set_nine_key(true);
    engine.set_input("64426");
    assert!(engine.pin_nine_key("ni"));
    let (cursor, marked) = tap(&mut engine, 2);
    assert_eq!(cursor, 2);
    assert!(!marked.chars().any(|c| c.is_ascii_digit()), "{marked}");
    engine.backspace();
    assert_eq!(engine.composition().text(), "64426");
    assert_eq!(engine.composition().cursor(), 2);
    assert_eq!(engine.nine_key_pinned(), 0);
    // 光标后那段读法不变
    let after = engine.query().unwrap().marked_text();
    assert_eq!(
        after.rsplit('\'').next(),
        marked.rsplit('\'').next(),
        "{after}"
    );
    // 再删光标前的键：剩下的 6 还偏向 n，光标后仍是原来的读法
    engine.backspace();
    assert_eq!(engine.composition().text(), "6426");
    let query = engine.query().unwrap();
    assert!(
        query.marked_text().starts_with('n'),
        "{}",
        query.marked_text()
    );
    assert_eq!(
        query.marked_text().rsplit('\'').next(),
        marked.rsplit('\'').next()
    );
}
