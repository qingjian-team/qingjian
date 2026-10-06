//! 容错、上屏消费、辅筛与会话隔离的原生引擎回归。

mod ice_support;

use {ice_support::Fixture, qingjian_core::EngineSession};

#[test]
fn date_keywords_keep_their_abbreviated_chinese_candidates() {
    let fixture = Fixture::new();
    fixture.write(
        "cn_dicts/words.dict.yaml",
        "---\nname: words\n...\n人群\tren qun\t1000\n",
    );
    let mut engine = fixture.engine();
    engine.set_input("rq");
    let candidates = engine.query().unwrap().candidates;
    assert!(candidates.items[0].text.contains('-'));
    let position = candidates
        .items
        .iter()
        .position(|c| c.text == "人群")
        .unwrap();
    assert!(position >= 5);
    assert_eq!(engine.commit(&candidates.items[position]), "人群");
    assert!(engine.composition().is_empty());
    engine.set_input("cC");
    assert!(
        engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .all(|c| !c.text.is_empty())
    );
}

#[test]
fn tolerant_spelling_consumes_the_original_keys() {
    let fixture = Fixture::new();
    fixture.write("cn_dicts/8105.dict.yaml", "---\nname: chars\n...\n去\tqu\t100\n知\tzhi\t100\n女\tnv\t100\n虐\tnve\t100\n你\tni\t100\n");
    let mut engine = fixture.engine();
    for (input, word) in [("qv", "去"), ("hzi", "知"), ("zih", "知"), ("nue", "虐")] {
        engine.set_input(input);
        let candidate = engine
            .query()
            .unwrap()
            .candidates
            .items
            .into_iter()
            .find(|c| c.text == word)
            .unwrap();
        assert_eq!(engine.commit(&candidate), word);
        assert!(engine.composition().is_empty(), "{input}");
    }
    engine.set_input("qvni");
    let candidate = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == "去")
        .unwrap();
    assert_eq!(engine.commit(&candidate), "去");
    assert_eq!(engine.composition().text(), "ni");
    assert!(fixture.path().is_dir());
}

#[test]
fn radical_auxiliary_filter_commits_without_leaving_the_code() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine();
    engine.set_input("xiu`ren");
    let candidates = engine.query().unwrap().candidates;
    assert!(candidates.items.iter().all(|c| c.text == "休"));
    assert_eq!(engine.commit(&candidates.items[0]), "休");
    assert!(engine.composition().is_empty());
    engine.set_input("xiu`kou");
    assert!(engine.query().unwrap().candidates.items.is_empty());
}

#[test]
fn correction_hints_and_four_decimal_places_are_preserved() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine();
    engine.set_input("geiyu");
    assert_eq!(
        engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .find(|c| c.text == "给予")
            .unwrap()
            .reading
            .as_deref(),
        Some("jǐ yǔ")
    );
    engine.set_input("R1234.5678");
    assert!(
        engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .any(|c| c.text == "壹仟贰佰叁拾肆元伍角陆分柒厘捌毫")
    );
}

#[test]
fn lunar_leap_months_and_future_dates_match_ice_reference() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine();
    for (input, word, comment) in [
        ("N20230322", "癸卯兔年闰二月初一", "星期三"),
        ("N20331222", "癸丑牛年闰十一月初一", "星期四"),
        ("N20510101", "庚午马年十一月十九", "星期日"),
        ("N21001231", "庚申猴年腊月初一", "星期五"),
    ] {
        engine.set_input(input);
        let candidate = &engine.query().unwrap().candidates.items[0];
        assert_eq!(candidate.text, word);
        assert_eq!(candidate.reading.as_deref(), Some(comment));
    }
    for (input, word) in [("cClog(2,8)", "3"), ("cCfrexp(8)", "0.5 * 2^4")] {
        engine.set_input(input);
        assert_eq!(engine.query().unwrap().candidates.items[0].text, word);
    }
}

#[test]
fn uuid_is_isolated_and_restored_between_input_contexts() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine();
    let mut saved = EngineSession::default();
    engine.set_input("uuid");
    let first = engine.query().unwrap().candidates.items[0].text.clone();
    engine.swap_session(&mut saved);
    engine.set_input("uuid");
    let second = engine.query().unwrap().candidates.items[0].text.clone();
    assert_ne!(first, second);
    engine.swap_session(&mut saved);
    assert_eq!(engine.query().unwrap().candidates.items[0].text, first);
    engine.backspace();
    engine.push('d');
    assert_ne!(engine.query().unwrap().candidates.items[0].text, first);
}

#[test]
fn ice_prefixes_do_not_capture_ordinary_english_or_symbol_selection() {
    let fixture = Fixture::new();
    fixture.write(
        "custom_phrase.txt",
        "由\tu\t1\n又\tu\t2\n有\tu\t3\n禁用\tu\t-1\n",
    );
    fixture.write(
        "en_dicts/en_ext.dict.yaml",
        "---\nname: ext\n...\nRust\trust\t100\nvery\tvery\t100\nUniversity\tuniversity\t100\n",
    );
    let mut engine = fixture.engine();
    engine.set_input("u");
    assert_eq!(
        engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .take(3)
            .map(|c| c.text.as_str())
            .collect::<Vec<_>>(),
        ["有", "又", "由"]
    );
    for (input, word) in [
        ("u", "有"),
        ("Rust", "Rust"),
        ("very", "very"),
        ("University", "University"),
    ] {
        engine.set_input(input);
        assert!(
            engine
                .query()
                .unwrap()
                .candidates
                .items
                .iter()
                .any(|c| c.text == word),
            "{input}"
        );
    }
    engine.set_input("vhelp");
    assert!(!engine.rime_ice_input_char('1'));
    assert!(!engine.expression_mode());
    engine.set_input("v1");
    assert!(!engine.rime_ice_input_char('2'));
}
