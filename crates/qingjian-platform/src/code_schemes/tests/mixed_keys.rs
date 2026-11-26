//! 混输按键规则由配置显式启用，旧配置不改变拼音输入。

use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;

use crate::{Config, SchemeSlot};

use super::{scratch, write};

fn configured(name: &str, rules: &str) -> Engine {
    let root = scratch(name);
    let user = root.join("user");
    write(
        &user.join("schemes/wubi86/scheme.toml"),
        "code_length = 4\n",
    );
    write(
        &user.join("schemes/wubi86/table.tsv"),
        "一\tggll\t100\n发\tv\t100\n甲\taaaa\t10\n乙\taaaa\t5\n",
    );
    let config: Config = toml::from_str(&format!(
        "[general]\nscheme = \"pinyin\"\ncode = \"wubi86\"\n[code.wubi86]\n{rules}"
    ))
    .unwrap();
    let mut engine = Engine::new(Dictionary::default());
    SchemeSlot::default().apply(&mut engine, &config, Some(&user), None);
    std::fs::remove_dir_all(root).unwrap();
    engine
}

#[test]
fn mixed_auto_commit_requires_four_keys_and_a_unique_word() {
    let mut engine = configured("mixed-auto", "mixed_auto_commit = true\n");
    for keys in ["v", "g", "gg", "ggl", "aaaa"] {
        engine.set_input(keys);
        assert_eq!(engine.commit_unique_code(), None, "{keys}");
    }
    engine.set_input("ggll");
    assert_eq!(engine.commit_unique_code().as_deref(), Some("一"));
}

#[test]
fn mixed_topping_is_opt_in_and_independent_of_auto_commit() {
    let mut engine = configured("mixed-top", "mixed_top = true\n");
    engine.set_input("ggll");
    assert_eq!(engine.commit_unique_code(), None);
    engine.set_input("aaaa");
    assert!(engine.code_tops('g'));
    assert!(!engine.code_tops(','));
}

#[test]
fn old_config_keeps_mixed_rules_disabled() {
    let mut engine = configured("mixed-default", "");
    engine.set_input("ggll");
    assert_eq!(engine.commit_unique_code(), None);
    assert!(!engine.code_tops('g'));
}
