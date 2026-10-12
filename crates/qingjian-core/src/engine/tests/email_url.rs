//! 邮箱 / 网址补全。

use super::*;

#[test]
fn email_at_suggests_domains_with_mode_hint() {
    let mut engine = engine();
    engine.set_input("qingjian@");
    let query = engine.query().unwrap();
    assert_eq!(query.mode_hint.as_deref(), Some("退出网址、邮箱模式"));
    let texts: Vec<_> = query.candidates.items.iter().map(|c| c.text.as_str()).collect();
    assert_eq!(texts[0], "qingjian@");
    assert!(texts.contains(&"qingjian@yeah.net"));
    assert!(texts.contains(&"qingjian@qq.com"));
}

#[test]
fn email_domain_prefix_filters() {
    let mut engine = engine();
    engine.set_input("qingjian@qq");
    let texts: Vec<_> = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .map(|c| c.text)
        .collect();
    assert!(texts.iter().any(|t| t == "qingjian@qq.com"));
    assert!(!texts.iter().any(|t| t == "qingjian@gmail.com"));
}

#[test]
fn email_url_mode_off_keeps_literal_only() {
    let mut engine = engine();
    engine.set_email_url_mode(false);
    engine.set_input("qingjian@");
    let query = engine.query().unwrap();
    assert!(query.mode_hint.is_none());
    assert_eq!(query.candidates.items.len(), 1);
    assert_eq!(query.candidates.items[0].text, "qingjian@");
}

#[test]
fn plain_pinyin_unaffected() {
    let mut engine = engine();
    engine.set_input("kaifa");
    let query = engine.query().unwrap();
    assert!(query.mode_hint.is_none());
    assert!(
        query
            .candidates
            .items
            .iter()
            .any(|c| c.kind == CandidateKind::Chinese && c.text.contains('开'))
    );
}

#[test]
fn www_dot_enters_url_mode() {
    let mut engine = engine();
    engine.set_input("www.");
    let query = engine.query().unwrap();
    assert_eq!(query.mode_hint.as_deref(), Some("退出网址、邮箱模式"));
    assert_eq!(query.candidates.items[0].text, "www.");
    assert!(query.candidates.items.len() > 1);
}
