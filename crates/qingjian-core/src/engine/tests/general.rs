//! 补充词库（通用大词库）：排序低主词库一档。

use super::*;

fn with_general(base: &str, general: &str, secondary: bool) -> Engine {
    let mut general = Dictionary::parse(general).unwrap();
    general.set_secondary(secondary);
    let mut engine = Engine::new(Dictionary::parse(base).unwrap());
    engine.set_extra_dictionaries(vec![general]);
    engine
}

fn first(engine: &mut Engine, input: &str) -> String {
    engine.set_input(input);
    engine.query().unwrap().candidates.items[0].text.clone()
}

/// 读音正好对上的片段（咋嫩）不挡住常用词的敲错纠正（`zanen` → 咱们）；当主词库的词就挡住了。
#[test]
fn fragments_do_not_block_typo_corrections() {
    let base = "咱们\tzan men\t20000\n咱\tzan\t5000\n赞\tzan\t8000\n们\tmen\t9000\n";
    let general = "咋嫩\tza3 nen4\t65\n";
    assert_eq!(
        first(&mut with_general(base, general, true), "zanen"),
        "咱们"
    );
    assert_eq!(
        first(&mut with_general(base, general, false), "zanen"),
        "咋嫩"
    );
}

/// 补充词库的长词不当前缀补全（`anzhao` 不出 按照规定），整段敲全了才出。
#[test]
fn long_words_need_their_full_reading() {
    let base = "按照\tan zhao\t9000\n安装\tan zhuang\t8000\n";
    let general = "按照规定\tan4 zhao4 gui1 ding4\t5000\n";
    let mut engine = with_general(base, general, true);
    assert!(!texts_of(&mut engine, "anzhao").contains(&"按照规定".to_owned()));
    assert_eq!(first(&mut engine, "anzhaoguiding"), "按照规定");
}

/// 词图选中的整词是补充词库的（只用）时排到最前，不被词级排序压在主词库生僻词（智永）后面。
#[test]
fn lattice_choice_from_general_goes_first() {
    let base = "智永\tzhi yong\t42\n只\tzhi\t90000\n用\tyong\t80000\n";
    let general = "只用\tzhi3 yong4\t1469\n";
    let mut engine = with_general(base, general, true);
    engine.set_input("zhiyong");
    let items = engine.query().unwrap().candidates.items;
    assert_eq!(items[0].text, "只用");
    assert_eq!(items.iter().filter(|c| c.text == "只用").count(), 1);
}

fn texts_of(engine: &mut Engine, input: &str) -> Vec<String> {
    engine.set_input(input);
    engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .map(|c| c.text)
        .collect()
}

/// 只认 咱们 的语言模型：主词库里模型没见过的词（青简词库清洗过的那批）不挡敲错纠正。
struct ZanmenModel;

impl LanguageModel for ZanmenModel {
    fn log_prob(&self, _previous: Option<&str>, word: &str) -> Option<f64> {
        (word == "咱们").then_some(-8.0)
    }
}

/// 主词库里读音正好对上、但语言模型没见过的词（赞恩）不挡 `zanen` → 咱们；模型认识它就挡。
#[test]
fn words_unknown_to_the_model_do_not_block_typo_corrections() {
    // 的 撑起总词频（真词库上亿），兜底分才是实际的量级
    let base = "咱们\tzan men\t20000\n赞恩\tzan4 en1\t300\n赞\tzan\t8000\n的\tde\t100000000\n";
    let mut engine =
        Engine::new(Dictionary::parse(base).unwrap()).with_language_model(Box::new(ZanmenModel));
    assert_eq!(first(&mut engine, "zanen"), "咱们");
    let mut engine = Engine::new(Dictionary::parse(base).unwrap());
    assert_eq!(first(&mut engine, "zanen"), "赞恩");
}
