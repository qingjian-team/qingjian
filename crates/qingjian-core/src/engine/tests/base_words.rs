//! 关掉基础词库：主词库只出单字，词全靠附加词库；再打开恢复。

use super::*;

#[test]
fn base_words_off_keeps_single_characters_and_extra_words() {
    let mut engine = Engine::new(
        Dictionary::parse("开发\tkai fa\t9000\n开\tkai\t20000\n发\tfa\t10000\n").unwrap(),
    );
    engine.set_extra_dictionaries(vec![Dictionary::parse("开阀\tkai fa\t100\n").unwrap()]);

    engine.set_base_words(false);
    engine.set_input("kaifa");
    let texts = texts_of(&engine);
    assert!(!texts.iter().any(|text| text == "开发"), "{texts:?}");
    assert!(texts.iter().any(|text| text == "开阀"), "{texts:?}");
    engine.set_input("kai");
    assert_eq!(texts_of(&engine).first().map(String::as_str), Some("开"));

    engine.set_base_words(true);
    engine.set_input("kaifa");
    assert_eq!(texts_of(&engine).first().map(String::as_str), Some("开发"));
}
