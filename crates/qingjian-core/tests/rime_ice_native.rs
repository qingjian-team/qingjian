//! 原生 Engine 的雾凇全拼行为，不调用 librime 或 Lua。

mod ice_support;

use ice_support::Fixture;
use qingjian_core::{CandidateKind, Engine};

fn query(engine: &mut Engine, input: &str) -> Vec<String> {
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

#[test]
fn full_pinyin_uses_ice_dictionary_and_commits_once() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine();
    engine.set_input("nihao");
    let candidates = engine.query().unwrap().candidates;
    let item = candidates.items.iter().find(|c| c.text == "你好").unwrap();
    assert_eq!(engine.commit(item), "你好");
    assert!(engine.composition().is_empty());
    assert!(query(&mut engine, "nh").contains(&"你好".to_owned()));
    assert!(query(&mut engine, "nihao").contains(&"👋".to_owned()));
}

#[test]
fn date_shortcuts_have_ice_order_and_formats() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine();
    let dates = query(&mut engine, "rq");
    assert_eq!(dates.len(), 5);
    assert!(dates[0].contains('-'));
    assert!(dates[1].contains('/'));
    assert!(dates[2].contains('.'));
    assert_eq!(dates[3].len(), 8);
    assert!(dates[4].contains('年'));
    for input in ["sj", "xq", "dt", "ts", "rqzh", "rqen", "nl"] {
        assert!(!query(&mut engine, input).is_empty(), "{input}");
    }
}

#[test]
fn lunar_new_year_and_invalid_date() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine();
    assert_eq!(query(&mut engine, "N20260217"), ["丙午马年正月初一"]);
    assert_eq!(query(&mut engine, "N20250229"), ["错误"]);
    assert_eq!(query(&mut engine, "N18991231"), ["错误"]);
    assert_eq!(query(&mut engine, "N2026"), ["输入完整的日期"]);
}

#[test]
fn calculator_functions_precedence_and_limits() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine();
    for (input, value) in [
        ("cC1+2", "3"),
        ("cCsin(pi/2)", "1"),
        ("cC-2^2", "-4"),
        ("cC5!", "120"),
        ("cCavg(1,2,3)", "2"),
        ("cC50%*2", "1"),
        ("cCsqrt(9)", "3"),
    ] {
        assert_eq!(query(&mut engine, input)[0], value, "{input}");
    }
    assert_eq!(query(&mut engine, "cC1/0"), ["1/0"]);
    let deep = format!("{}1{}", "(".repeat(100), ")".repeat(100));
    assert_eq!(query(&mut engine, &format!("cC{deep}")), [deep]);
}

#[test]
fn money_unicode_symbols_and_radicals() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine();
    assert!(query(&mut engine, "R1234.56").contains(&"壹仟贰佰叁拾肆元伍角陆分".to_owned()));
    assert_eq!(query(&mut engine, "U4e2d"), ["中"]);
    assert!(query(&mut engine, "Ud800").is_empty());
    assert_eq!(query(&mut engine, "va"), ["ā", "á", "ǎ", "à"]);
    assert_eq!(query(&mut engine, "v1"), ["一", "壹", "①"]);
    engine.set_input("uUrenmu");
    let list = engine.query().unwrap().candidates;
    assert_eq!(list.items[0].text, "休");
    assert_eq!(list.items[0].reading.as_deref(), Some("xiu"));
}

#[test]
fn uuid_is_stable_per_input_and_new_after_commit() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine();
    engine.set_input("uuid");
    let first = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(first.text.len(), 36);
    assert_eq!(first.text, engine.query().unwrap().candidates.items[0].text);
    assert_eq!(engine.commit(&first), first.text);
    assert_ne!(query(&mut engine, "uuid")[0], first.text);
}

#[test]
fn phrases_pins_english_and_mixed_words() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine();
    assert_eq!(query(&mut engine, "qj")[0], "青简");
    assert_eq!(query(&mut engine, "d")[0], "的");
    assert!(query(&mut engine, "hello").contains(&"hello".to_owned()));
    assert!(query(&mut engine, "Hello").contains(&"Hello".to_owned()));
    assert!(query(&mut engine, "HELLO").contains(&"HELLO".to_owned()));
    assert!(query(&mut engine, "xguang").contains(&"X光".to_owned()));
}

#[test]
fn profile_is_optional_and_does_not_change_other_schemes() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine();
    engine.set_shuangpin(Some(qingjian_core::ShuangpinScheme::Xiaohe));
    assert!(!engine.rime_ice_active());
    engine.set_shuangpin(None);
    assert!(engine.rime_ice_active());
    engine.set_english_mode(true);
    assert!(!engine.rime_ice_active());
    assert!(engine.rime_ice_available());
    engine.set_english_mode(false);
    assert!(!engine.rime_ice_input_char('U'));
    engine.set_input("R12");
    assert!(engine.rime_ice_input_char('3'));
    engine.disable_rime_ice();
    assert!(query(&mut engine, "yuanyinqing").contains(&"原引擎".to_owned()));
}

#[test]
fn failed_reload_preserves_the_current_profile() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine();
    assert!(
        engine
            .load_rime_ice(
                &fixture.path().join("missing"),
                &fixture.path().join("cache")
            )
            .is_err()
    );
    assert!(query(&mut engine, "nihao").contains(&"你好".to_owned()));
    engine.set_input("rq");
    assert!(
        engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .all(|c| c.kind == CandidateKind::Shortcut)
    );
}

#[test]
#[ignore = "需要用户提供的原始雾凇数据"]
fn original_ice_data_runs_through_the_rust_engine() {
    let source = std::env::var_os("QINGJIAN_TEST_ICE_DATA").unwrap();
    let cache = std::env::var_os("QINGJIAN_TEST_ICE_CACHE").unwrap();
    let mut engine = Engine::new(qingjian_dictionary::Dictionary::parse("").unwrap());
    engine
        .load_rime_ice(std::path::Path::new(&source), std::path::Path::new(&cache))
        .unwrap();
    for (input, word) in [
        ("nihao", "你好"),
        ("N20260217", "丙午马年正月初一"),
        ("U4e2d", "中"),
        ("cCsin(pi/2)", "1"),
        ("va", "ā"),
        ("xguang", "X光"),
    ] {
        assert!(
            query(&mut engine, input).contains(&word.to_owned()),
            "{input}"
        );
    }
    for input in [
        "rq", "sj", "xq", "dt", "ts", "rqzh", "rqen", "nl", "uuid", "vhelp", "R1234.56", "uUrenmu",
    ] {
        assert!(!query(&mut engine, input).is_empty(), "{input}");
    }
    let radicals = query(&mut engine, "uUrenmu");
    assert!(
        radicals.iter().position(|w| w == "休").unwrap()
            < radicals.iter().position(|w| w == "㑄").unwrap()
    );
}
