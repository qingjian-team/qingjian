//! 双拼模糊音的查询与上屏回归。

use qingjian_core::{Engine, FuzzyRules, ShuangpinScheme};
use qingjian_dictionary::Dictionary;

#[test]
fn fuzzy_shuangpin_accepts_suang_and_consumes_only_its_keys() {
    for scheme in ShuangpinScheme::ALL {
        let dictionary =
            Dictionary::parse("双\tshuang\t9000\n人\tren\t8000\n双人\tshuang ren\t12000\n")
                .unwrap();
        let mut engine = Engine::new(dictionary);
        engine.set_shuangpin(Some(scheme));
        engine.set_fuzzy(FuzzyRules {
            s_sh: true,
            ..FuzzyRules::default()
        });
        let keys = format!("s{}", scheme.encode("shuang").unwrap()[1]);
        let remaining: String = scheme.encode("ren").unwrap().iter().collect();
        engine.set_input(&keys);
        let query = engine.query().expect("s/sh 模糊音应让对应的双拼键查询到双");
        let candidate = query
            .candidates
            .items
            .iter()
            .find(|c| c.text == "双")
            .expect("缺少双")
            .clone();
        assert_eq!(engine.commit(&candidate), "双");
        assert!(engine.composition().is_empty());
        for separator in ["", "'"] {
            engine.set_input(&format!("{keys}{separator}{remaining}"));
            let query = engine.query().unwrap();
            assert!(
                query.candidates.items.iter().any(|c| c.text == "双人"),
                "{scheme}"
            );
            let candidate = query
                .candidates
                .items
                .iter()
                .find(|c| c.text == "双")
                .unwrap()
                .clone();
            assert_eq!(engine.commit(&candidate), "双");
            assert_eq!(engine.composition().text(), remaining, "{scheme}");
        }
    }
}

#[test]
fn fuzzy_pair_keeps_semicolon_final_available_for_the_next_syllable() {
    for scheme in [ShuangpinScheme::Microsoft, ShuangpinScheme::Sogou] {
        let dictionary = Dictionary::parse("双星\tshuang xing\t9000\n").unwrap();
        let mut engine = Engine::new(dictionary);
        engine.set_shuangpin(Some(scheme));
        engine.set_fuzzy(FuzzyRules {
            s_sh: true,
            ..FuzzyRules::default()
        });
        engine.set_input("sdx");
        assert!(engine.takes_semicolon());
        engine.push(';');
        let query = engine.query().unwrap();
        let candidate = query
            .candidates
            .items
            .iter()
            .find(|c| c.text == "双星")
            .unwrap();
        assert_eq!(engine.commit(candidate), "双星");
        assert!(engine.composition().is_empty());
    }
}

#[test]
fn fuzzy_toggle_does_not_admit_invalid_pairs_or_change_normal_decoding() {
    let dictionary =
        Dictionary::parse("双\tshuang\t9000\n三\tsan\t8000\n思\tsi\t7000\n是\tshi\t6000\n")
            .unwrap();
    let mut engine = Engine::new(dictionary);
    engine.set_shuangpin(Some(ShuangpinScheme::Ziranma));
    for enabled in [false, true, false, true] {
        engine.set_fuzzy(FuzzyRules {
            s_sh: enabled,
            ..FuzzyRules::default()
        });
        engine.set_input("sd");
        let found = engine
            .query()
            .is_ok_and(|q| q.candidates.items.iter().any(|c| c.text == "双"));
        assert_eq!(found, enabled);
        engine.set_input("sj");
        assert_eq!(engine.query().unwrap().candidates.items[0].text, "三");
        engine.set_input("si");
        let query = engine.query().unwrap();
        assert_eq!(query.candidates.items[0].text, "思");
        assert_eq!(
            query.candidates.items.iter().any(|c| c.text == "是"),
            enabled
        );
        engine.set_input("bd");
        assert!(engine.query().is_err());
    }
    engine.set_fuzzy(FuzzyRules {
        n_l: true,
        ..FuzzyRules::default()
    });
    engine.set_input("sd");
    assert!(engine.query().is_err());
}
