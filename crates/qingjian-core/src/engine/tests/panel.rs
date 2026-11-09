//! 符号面板。

use super::*;
use crate::panel::{CATALOG, PanelGroup, PanelHistory, PanelUse};
use std::sync::{Arc, Mutex};

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

/// 记在内存里的面板使用记录，`recorded` 留给测试看记了哪些。
struct MemoryPanelHistory {
    uses: Vec<PanelUse>,

    recorded: Arc<Mutex<Vec<String>>>,
}

impl PanelHistory for MemoryPanelHistory {
    fn uses(&self) -> &[PanelUse] {
        &self.uses
    }

    fn record(&mut self, item: &str) {
        self.recorded.lock().unwrap().push(item.to_owned());
        match self.uses.iter_mut().find(|entry| entry.item == item) {
            Some(entry) => entry.count += 1,
            None => self.uses.push(PanelUse {
                item: item.to_owned(),
                count: 1,
                last: 0,
            }),
        }
    }
}

#[test]
fn panel_commits_are_recorded_and_lead_the_common_section() {
    let dictionary = Dictionary::parse("哭\tku\t100\n").unwrap();
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let mut engine = Engine::new(dictionary)
        .with_emoji(EmojiTable::default())
        .with_panel_history(Box::new(MemoryPanelHistory {
            uses: Vec::new(),
            recorded: recorded.clone(),
        }));
    let star = CATALOG.item("★").unwrap().clone();
    assert!(engine.open_symbol_panel());
    assert_ne!(engine.symbol_panel().unwrap().section().items[0].text, "★");
    assert_eq!(engine.commit(&star), "★");
    // 重新打开，「常用」第一个就是刚用过的
    assert!(engine.open_symbol_panel());
    let panel = engine.symbol_panel().unwrap();
    assert_eq!(panel.section().name, "常用");
    assert_eq!(panel.section().items[0].text, "★");

    // 面板没开时上屏的候选不记
    engine.close_symbol_panel();
    engine.set_input("ku");
    let first = engine.query().unwrap().candidates.items[0].clone();
    engine.commit(&first);
    // 私密输入时从面板上屏也不记
    engine.set_private(true);
    assert!(engine.open_symbol_panel());
    engine.commit(&star);
    assert_eq!(*recorded.lock().unwrap(), ["★"]);
}
