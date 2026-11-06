//! 修改方案描述中的组词规则会重新装配，不能被码表复用吞掉。

use super::{config, scratch, write};
use crate::SchemeSlot;
use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;

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
