//! 修改方案描述中的组词规则会重新装配，不能被码表复用吞掉。

use super::{config, scratch, write};
use crate::SchemeSlot;
use crate::code_schemes::{WUBI86, find};
use qingjian_core::Engine;
use qingjian_dictionary::{CodeTable, Dictionary};

#[test]
fn only_the_packed_bundled_wubi_has_an_implicit_phrase_rule() {
    let root = scratch("packed-phrase-rule");
    let bundled = root.join("bundled");
    std::fs::create_dir_all(&bundled).unwrap();
    let table = CodeTable::parse("甲\tabcd\t1\n乙\tefgh\t1\n").unwrap();
    for id in [WUBI86, "custom"] {
        table
            .write_qj(&bundled.join(format!("{id}.qj")), &Default::default())
            .unwrap();
    }
    let mut engine = Engine::new(Dictionary::default());
    SchemeSlot::default().apply(&mut engine, &config("none", WUBI86), None, Some(&bundled));
    assert_eq!(
        engine.code_scheme().unwrap().phrase_code("甲乙").as_deref(),
        Some("abef")
    );
    assert!(
        !find("custom", None, Some(&bundled))
            .unwrap()
            .load()
            .unwrap()
            .wubi_phrases
    );
    write(&root.join("schemes/wubi86/scheme.toml"), "");
    write(
        &root.join("schemes/wubi86/table.tsv"),
        "甲\tabcd\t1\n乙\tefgh\t1\n",
    );
    assert!(
        !find(WUBI86, Some(&root), Some(&bundled))
            .unwrap()
            .load()
            .unwrap()
            .wubi_phrases
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn phrase_rule_changes_are_applied_without_touching_the_table() {
    let root = scratch("phrase-rule");
    let descriptor = root.join("schemes/custom/scheme.toml");
    write(
        &root.join("schemes/custom/table.tsv"),
        "甲\tabcd\t10\n乙\tefgh\t10\n",
    );
    let mut engine = Engine::new(Dictionary::default());
    let mut slot = SchemeSlot::default();
    for (description, enabled) in [
        ("", false),
        ("phrase_rule = \"wubi86\"\n", true),
        ("", false),
    ] {
        write(&descriptor, description);
        slot.apply(&mut engine, &config("none", "custom"), Some(&root), None);
        assert_eq!(engine.code_scheme().unwrap().wubi_phrases, enabled);
    }
    std::fs::remove_dir_all(root).unwrap();
}
