//! 英文直输段里的「用户名@」后面跟常见邮箱地址。

use super::engine;
use crate::CandidateKind;

#[test]
fn username_and_at_bring_whole_addresses_after_the_raw_text() {
    let mut engine = engine();
    engine.set_input("qingjian@");
    let items = engine.query().unwrap().candidates.items;
    assert_eq!(items[0].text, "qingjian@");
    assert_eq!(items[1].text, "qingjian@qq.com");
    assert_eq!(items[1].kind, CandidateKind::Shortcut);

    engine.set_input("zhang.san@g");
    let texts: Vec<String> = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .map(|c| c.text)
        .collect();
    assert_eq!(texts, ["zhang.san@g", "zhang.san@gmail.com"]);
}

#[test]
fn picking_an_address_commits_it_whole() {
    let mut engine = engine();
    engine.set_input("qingjian@ou");
    let items = engine.query().unwrap().candidates.items;
    assert_eq!(engine.commit(&items[1]), "qingjian@outlook.com");
    assert!(engine.composition().is_empty());
}

#[test]
fn no_addresses_without_a_username_or_after_a_full_domain() {
    let mut engine = engine();
    for input in ["@", "qingjian@qq.com", "a.b"] {
        engine.set_input(input);
        assert_eq!(engine.query().unwrap().candidates.items.len(), 1, "{input}");
    }
}
