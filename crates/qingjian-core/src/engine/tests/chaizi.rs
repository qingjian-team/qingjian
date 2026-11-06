//! 问字模式：部件读音、笔画、两者混着打；候选带读音；码点与英文词照旧。

use super::*;

fn first_texts(engine: &Engine, n: usize) -> Vec<String> {
    texts_of(engine).into_iter().take(n).collect()
}

#[test]
fn part_readings_find_the_character_with_its_reading() {
    let mut engine = engine();
    engine.set_input("umumu");
    let query = engine.query().unwrap();
    let lin = &query.candidates.items[0];
    assert_eq!(lin.text, "林");
    assert_eq!(lin.reading.as_deref(), Some("lín"));
    assert_eq!(query.marked_text(), "umu'mu");
    // 讠舌 也读 yan she：同一档按字频，话 在 谢 前
    engine.set_input("uyanshe");
    assert_eq!(first_texts(&engine, 2), ["话", "谢"]);
    engine.set_input("uniuniuniu");
    assert_eq!(first_texts(&engine, 1), ["犇"]);
    // 敲到一半也出：前缀 mumu 的 森（mumumu）跟在 林 后面
    engine.set_input("umumu");
    assert!(texts_of(&engine).contains(&"森".to_owned()));
}

/// 整字笔顺：`木` 是 `hspn`（横竖撇捺），`林` 是两遍。
#[test]
fn strokes_find_the_character() {
    let mut engine = engine();
    engine.set_input("uhspn");
    assert_eq!(first_texts(&engine, 1), ["木"]);
    engine.set_input("uhspnhspn");
    assert_eq!(first_texts(&engine, 1), ["林"]);
    // 打一半也出，按「还差几笔」排
    engine.set_input("uhsp");
    assert_eq!(first_texts(&engine, 1), ["木"]);
}

/// 倒笔画：整串反过来打，`木` 是 `npsh`。
#[test]
fn reversed_strokes_find_the_character() {
    let mut engine = engine();
    engine.set_input("unpsh");
    assert_eq!(first_texts(&engine, 1), ["木"]);
    // 林 = hspnhspn，倒过来是 npshnpsh
    engine.set_input("unpshnpsh");
    assert_eq!(first_texts(&engine, 1), ["林"]);
}

/// 混输：一个部件按读音、另一个按笔画。
#[test]
fn readings_and_strokes_mix_within_one_input() {
    let mut engine = engine();
    // 木(mu) + 木的笔顺
    engine.set_input("umuhspn");
    assert_eq!(first_texts(&engine, 1), ["林"]);
    // 禾(he) + 口(szh)
    engine.set_input("uheszh");
    assert_eq!(first_texts(&engine, 1), ["和"]);
    // 第一个部件按笔顺、第二个按读音：禾(phspn) + 口(kou)
    engine.set_input("uphspnkou");
    assert_eq!(first_texts(&engine, 1), ["和"]);
}

/// 部件的俗名：囗 的字表读音是 wei，用户按 kou 打也要认。
#[test]
fn part_alias_reads_the_shape() {
    let mut engine = engine();
    engine.set_input("ukoumu");
    assert_eq!(first_texts(&engine, 1), ["困"]);
    engine.set_input("ukouhspn");
    assert_eq!(first_texts(&engine, 1), ["困"]);
    // 本来的读音照旧
    engine.set_input("uweimu");
    assert_eq!(first_texts(&engine, 1), ["困"]);
}

/// 繁体拆法并进了简体主键：打台北字形的拆法（辶 言）也能找到简体字。
#[test]
fn traditional_splits_find_the_simplified_character() {
    let mut engine = engine();
    // 这 = 辶 文（简体字形）或 辶 言（繁体字形），两种都出同一个字
    engine.set_input("uzouyan");
    assert_eq!(first_texts(&engine, 1), ["这"]);
    engine.set_input("uzouwen");
    assert_eq!(first_texts(&engine, 1), ["这"]);
}

#[test]
fn hex_looking_letters_select_instead_of_typing_a_code_point() {
    let mut engine = engine();
    engine.set_input("udada");
    assert!(!engine.unicode_entry());
    assert_eq!(first_texts(&engine, 1), ["夶"]);
    // 有数字的仍是码点
    engine.set_input("u4e00");
    assert!(engine.unicode_entry());
    assert_eq!(first_texts(&engine, 1), ["一"]);
    // 拆不出字的十六进制字母仍可能是码点：`face` 切得出部件读音，但那是拼音不是拆字
    engine.set_input("uface");
    assert!(engine.unicode_entry());
}

#[test]
fn english_words_stay_first() {
    let words = WordList::parse("under\tunder\t3000\n").unwrap();
    let mut engine = engine().with_english(words);
    engine.set_input("under");
    assert_eq!(first_texts(&engine, 1), ["under"]);
}
