//! 形码方案下的附加候选：emoji 按上屏的字词配，`rq` / `sj` / `xq` 的快捷写法排在打全编码的字词后面。

use qingjian_dictionary::CodeTable;

use super::super::*;
use super::{code_texts, wubi};

#[test]
fn code_words_bring_their_emoji() {
    // #927：五笔打出 开 也配上它的 emoji，选了 emoji 吃掉整段编码
    let mut engine = wubi().with_emoji(EmojiTable::parse("开\t🔓\n").unwrap());
    assert_eq!(code_texts(&mut engine, "ga"), ["开", "🔓", "开发"]);
    let emoji = engine.query().unwrap().candidates.items.remove(1);
    assert_eq!(emoji.kind, CandidateKind::Emoji);
    assert_eq!(engine.commit(&emoji), "🔓");
    assert!(engine.composition().is_empty());
}

#[test]
fn date_shortcuts_follow_the_finished_codes() {
    // #906：rq 也是 的（rqyy）的编码前缀：日期写法排在打全 rq 的字之后、只命中前缀的字之前
    let mut engine = wubi();
    engine.set_code_scheme(Some(CodeScheme::new(
        "wubi",
        CodeTable::parse("盾\trq\t100\n的\trqyy\t90000\n").unwrap(),
    )));
    engine.set_input("rq");
    let items = engine.query().unwrap().candidates.items;
    assert_eq!(items[0].text, "盾");
    assert_eq!(items[1].kind, CandidateKind::Shortcut);
    assert_eq!(items.last().unwrap().text, "的");
    assert!(
        items
            .iter()
            .any(|c| c.kind == CandidateKind::Shortcut && c.text.contains('-'))
    );
}
