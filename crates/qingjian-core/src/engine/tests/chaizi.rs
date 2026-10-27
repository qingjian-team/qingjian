//! 问字模式的拆字：`umumu` → 林，候选带读音；码点与英文词照旧。

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
    // 拆不出字的十六进制字母仍可能是码点
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
