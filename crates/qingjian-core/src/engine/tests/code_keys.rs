//! 形码的按键规则：满码唯一自动上屏与顶字。

use qingjian_dictionary::CodeTable;

use super::code::{CODES, wubi};
use super::*;

/// 只用形码、两条规则都开着。码表里多加两条同码的 工 / 式，满码也不唯一；
/// 再加一条写错的 5 码（极点码表里真有），满码仍是 4。
fn strict() -> Engine {
    let mut engine = wubi();
    let source = format!("{CODES}工\taaaa\t10\n式\taaaa\t5\nTF卡\tttfhh\t1\n");
    let mut scheme = CodeScheme::new("wubi", CodeTable::parse(&source).unwrap());
    scheme.auto_commit = true;
    scheme.top = true;
    engine.set_code_scheme(Some(scheme));
    engine
}

/// 一键一键敲，每键之后照壳的做法试一次自动上屏；返回最后一次上屏的文本。
fn type_keys(engine: &mut Engine, keys: &str) -> Option<String> {
    let mut committed = None;
    for c in keys.chars() {
        engine.push(c);
        if let Some(text) = engine.commit_unique_code() {
            committed = Some(text);
        }
    }
    committed
}

#[test]
fn a_unique_full_code_commits_itself() {
    let mut engine = strict();
    engine.push('g');
    engine.push('g');
    engine.push('l');
    assert_eq!(engine.commit_unique_code(), None);
    engine.push('l');
    assert_eq!(engine.commit_unique_code().as_deref(), Some("一"));
    assert!(engine.composition().is_empty());
}

#[test]
fn a_full_code_shared_by_two_words_waits_for_a_choice() {
    let mut engine = strict();
    assert_eq!(type_keys(&mut engine, "aaaa"), None);
    assert_eq!(engine.composition().text(), "aaaa");
}

#[test]
fn auto_commit_follows_the_switch_and_stays_out_of_mixed_input() {
    let mut engine = wubi();
    assert_eq!(type_keys(&mut engine, "ggll"), None);

    // 混输下 `ggll` 之后可能还要接着打拼音，不能替用户上屏
    let mut engine = strict();
    engine.set_phonetic(true);
    assert_eq!(type_keys(&mut engine, "ggll"), None);
}

#[test]
fn a_letter_after_a_full_code_tops_the_highlighted_word() {
    let mut engine = strict();
    assert_eq!(type_keys(&mut engine, "aaaa"), None);
    assert!(engine.code_tops('g'));
    // 只有字母顶字：数字是选词，标点另有一套上屏规则
    assert!(!engine.code_tops('1'));
    assert!(!engine.code_tops(','));

    engine.clear();
    type_keys(&mut engine, "gan");
    assert!(!engine.code_tops('t'));
}

#[test]
fn an_empty_full_code_tops_nothing() {
    // 满码但没有一个候选（敲错了）：顶了也只会把字母原样上屏，不如留着让用户退格
    let mut engine = strict();
    type_keys(&mut engine, "zzzz");
    assert!(!engine.code_tops('g'));
}

#[test]
fn topping_stays_out_of_mixed_input_and_follows_the_switch() {
    let mut engine = strict();
    engine.set_phonetic(true);
    type_keys(&mut engine, "aaaa");
    assert!(!engine.code_tops('g'));

    let mut engine = wubi();
    engine.set_input("gant");
    assert!(!engine.code_tops('g'));
}

#[test]
fn the_log_names_the_code_scheme() {
    let mut engine = strict();
    assert_eq!(engine.scheme_key(), "wubi");
    engine.set_code_scheme(Some(CodeScheme::new(
        "huma",
        CodeTable::parse(CODES).unwrap(),
    )));
    assert_eq!(engine.scheme_key(), "huma");
    engine.set_phonetic(true);
    assert_eq!(engine.scheme_key(), "pinyin+huma");
}

#[test]
fn only_plain_code_input_counts_as_composing_code() {
    let mut engine = strict();
    assert!(!engine.composing_code());
    engine.set_input("zz");
    assert!(engine.composing_code());
    engine.set_input("no-way");
    assert!(!engine.composing_code());
    engine.set_phonetic(true);
    engine.set_input("ggll");
    assert!(!engine.composing_code());
}
