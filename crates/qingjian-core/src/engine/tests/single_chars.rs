//! 单字模式（#848）：组句中切进去候选只剩单字，只管这一段组句。

use super::*;

fn nihao_engine() -> Engine {
    let dictionary = "你好\tni hao\t9000\n拟好\tni hao\t10\n你\tni\t9000\n尼\tni\t500\n好\thao\t9000\n号\thao\t800\n";
    Engine::new(Dictionary::parse(dictionary).unwrap())
}

#[test]
fn only_single_chars_until_the_composition_ends() {
    let mut engine = nihao_engine();
    assert!(!engine.toggle_single_chars(), "没在组句时不进");
    engine.set_input("nihao");
    assert!(texts_of(&engine).iter().any(|t| t == "你好"));
    assert!(engine.toggle_single_chars());
    let texts = texts_of(&engine);
    assert!(!texts.is_empty());
    assert!(texts.iter().all(|t| t.chars().count() == 1), "{texts:?}");
    assert_eq!(texts[0], "你");

    // 选了 你，剩下的 hao 还在这段组句里，接着只出单字
    let ni = engine.query().unwrap().candidates.items[0].clone();
    engine.commit(&ni);
    assert_eq!(engine.composition().text(), "hao");
    assert!(texts_of(&engine).iter().all(|t| t.chars().count() == 1));

    // 上屏完，下一段回到正常候选
    let hao = engine.query().unwrap().candidates.items[0].clone();
    engine.commit(&hao);
    assert!(engine.composition().is_empty());
    engine.set_input("nihao");
    assert!(texts_of(&engine).iter().any(|t| t == "你好"));
}

#[test]
fn toggling_again_or_clearing_restores_the_words() {
    let mut engine = nihao_engine();
    engine.set_input("nihao");
    engine.toggle_single_chars();
    engine.toggle_single_chars();
    assert!(texts_of(&engine).iter().any(|t| t == "你好"));

    engine.toggle_single_chars();
    engine.clear();
    engine.set_input("nihao");
    assert!(texts_of(&engine).iter().any(|t| t == "你好"));

    // 退格删空也算这段组句结束
    engine.toggle_single_chars();
    while engine.backspace() {}
    for c in "nihao".chars() {
        engine.push(c);
    }
    assert!(texts_of(&engine).iter().any(|t| t == "你好"));
}
