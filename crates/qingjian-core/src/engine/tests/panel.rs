//! 符号面板。

use super::*;
use crate::panel::PanelGroup;

#[test]
fn panel_searches_the_typed_words_and_commits_the_whole_input() {
    let dictionary = Dictionary::parse("哭\tku\t100\n箭头\tjian tou\t100\n").unwrap();
    let mut engine = Engine::new(dictionary).with_emoji(EmojiTable::default());
    // 没在组句（从菜单打开）：没有搜索组，上屏不碰缓冲区
    assert!(engine.open_symbol_panel());
    let panel = engine.symbol_panel().unwrap();
    assert_eq!(panel.group(), PanelGroup::Symbols);
    let first = panel.section().items[0].clone();
    assert_eq!(engine.commit(&first), first.text);
    assert!(engine.symbol_panel().is_none());

    engine.set_input("ku");
    assert!(engine.open_symbol_panel());
    let panel = engine.symbol_panel().unwrap();
    assert_eq!(panel.group(), PanelGroup::Search);
    assert_eq!(panel.section().name, "ku");
    let texts: Vec<&str> = panel
        .section()
        .items
        .iter()
        .map(|c| c.text.as_str())
        .collect();
    assert!(texts.contains(&"(T_T)"), "{texts:?}");
    assert!(texts.contains(&"😭"), "{texts:?}");
    let sob = panel
        .section()
        .items
        .iter()
        .find(|c| c.text == "😭")
        .unwrap()
        .clone();
    assert_eq!(sob.kind, CandidateKind::Emoji);
    assert_eq!(sob.reading.as_deref(), Some("放声大哭"));
    assert_eq!(engine.commit(&sob), "😭");
    assert!(engine.composition().is_empty());
    assert!(engine.symbol_panel().is_none());

    engine.set_input("jiantou");
    assert!(engine.open_symbol_panel());
    let texts: Vec<String> = engine
        .symbol_panel()
        .unwrap()
        .section()
        .items
        .iter()
        .map(|c| c.text.clone())
        .collect();
    assert!(texts.contains(&"→".to_owned()), "{texts:?}");
    engine.clear();
    assert!(engine.symbol_panel().is_none());
}
