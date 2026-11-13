//! 后台准备方案后，输入线程不再访问方案文件。

use qingjian_core::{CodeScheme, Engine};
use qingjian_dictionary::{CodeTable, Dictionary};

use crate::{CodeRules, SchemeSlot};

use super::{config, scratch, write};

#[test]
fn prepared_code_can_be_installed_after_files_are_removed() {
    let root = scratch("prepared-code");
    let user = root.join("user");
    write(&user.join("schemes/huma/scheme.toml"), "");
    write(&user.join("schemes/huma/table.tsv"), "一\tyi\t1\n");
    let mut slot = SchemeSlot::default();
    let mut engine = Engine::new(Dictionary::default());
    let prepared = slot.prepare(&config("none", "huma"), Some(&user), None, false);
    std::fs::remove_dir_all(root).unwrap();
    slot.apply_prepared(&mut engine, prepared);
    assert_eq!(slot.code_label(), Some("huma"));
    assert_eq!(engine.code_scheme().unwrap().table.len(), 1);
    assert!(!engine.is_phonetic());
}

#[test]
fn prepared_rules_preserve_the_loaded_table() {
    let root = scratch("prepared-rules");
    let user = root.join("user");
    write(&user.join("schemes/huma/scheme.toml"), "");
    write(&user.join("schemes/huma/table.tsv"), "一\tyi\t1\n");
    let mut slot = SchemeSlot::default();
    let mut engine = Engine::new(Dictionary::default());
    let mut config = config("none", "huma");
    slot.apply(&mut engine, &config, Some(&user), None);
    engine.set_code_scheme(Some(CodeScheme::new(
        "marker",
        CodeTable::parse("一\tyi\t1\n").unwrap(),
    )));
    config.code.insert(
        "huma".into(),
        CodeRules {
            auto_commit: Some(false),
            top: Some(true),
            mixed_auto_commit: Some(true),
            mixed_top: Some(true),
        },
    );
    let prepared = slot.prepare(&config, Some(&user), None, true);
    std::fs::remove_dir_all(root).unwrap();
    slot.apply_prepared(&mut engine, prepared);
    let scheme = engine.code_scheme().unwrap();
    assert_eq!(scheme.key, "marker");
    assert!(!scheme.auto_commit);
    assert!(scheme.top);
    assert!(scheme.mixed_auto_commit);
    assert!(scheme.mixed_top);
}
