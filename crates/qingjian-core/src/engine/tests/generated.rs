//! 生成型快捷候选：UUID（`vuuid`、全拼下的 `uuid`、双拼的 `Vuuid`）与时间戳（`vts`）。

use super::*;

#[test]
fn expression_uuid_gives_four_forms_and_digits_select() {
    let mut engine = engine();
    engine.set_input("vuuid");
    let first = texts_of(&engine);
    assert_eq!(first.len(), 4, "{first:?}");
    assert_eq!(first[2], first[0].to_ascii_uppercase());
    // 壳每次刷新都重查：同一段组句里不变
    assert_eq!(texts_of(&engine), first);
    // 数字是选词键，不进算式；其余表达式照旧吃数字
    assert!(!engine.expression_input('2'));
    let pick = engine.query().unwrap().candidates.items[1].clone();
    assert_eq!(engine.commit(&pick), first[1]);
    engine.set_input("vuui");
    assert!(engine.expression_input('2'));

    // 新一段组句换一组
    engine.set_input("vuuid");
    assert_ne!(texts_of(&engine)[0], first[0]);
}

#[test]
fn bare_uuid_follows_the_english_word_in_question_mode() {
    let words = WordList::parse("uuid\tuuid\t1710\n").unwrap();
    let mut engine = engine().with_english(words);
    engine.set_input("uuid");
    assert!(engine.question_mode());
    let texts = texts_of(&engine);
    assert_eq!(texts[0], "uuid");
    assert_eq!(texts.len(), 5, "{texts:?}");
    assert!(!engine.unicode_entry());
}

#[test]
fn shuangpin_uses_shifted_expression_key() {
    let mut engine = xiaohe();
    engine.set_input("uuid");
    assert!(!texts_of(&engine).iter().any(|text| text.len() == 36));
    engine.set_input("Vuuid");
    assert_eq!(texts_of(&engine).len(), 4);
}

#[test]
fn expression_timestamp_is_stable_within_a_composition() {
    let mut engine = engine();
    engine.set_input("vts");
    let first = texts_of(&engine);
    assert_eq!(first.len(), 4, "{first:?}");
    assert!(first[0].bytes().all(|b| b.is_ascii_digit()));
    assert!(first[1].ends_with('Z'));
    assert!(!engine.expression_input('1'));
    std::thread::sleep(std::time::Duration::from_millis(5));
    assert_eq!(texts_of(&engine), first);
    // 前缀上的算式不受影响
    engine.set_input("vt");
    assert!(engine.expression_input('1'));
}
