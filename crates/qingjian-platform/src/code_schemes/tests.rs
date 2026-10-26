use std::path::{Path, PathBuf};

use qingjian_core::{CodeScheme, Engine};
use qingjian_dictionary::{CodeTable, Dictionary};

use super::*;
use crate::{CodeRules, Config, SchemeSlot};

/// 每个测试一个干净的临时目录。
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qingjian-code-schemes-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// 随包的 `schemes` 目录，里面只有五笔。
fn bundled(root: &Path) -> PathBuf {
    let dir = root.join("bundled/schemes");
    write(&dir.join("wubi86/scheme.toml"), "name = \"五笔（86）\"\n");
    write(&dir.join("wubi86/table.tsv"), "一\tggll\t1\n");
    dir
}

#[test]
fn ids_cannot_collide_with_the_pinyin_side() {
    assert!(is_valid_id("huma"));
    assert!(is_valid_id("xiaohe-yinxing"));
    assert!(is_valid_id("wubi98"));
    assert!(is_valid_id("wubi86"));
    assert!(!is_valid_id(""));
    assert!(!is_valid_id("wubi"));
    assert!(!is_valid_id("xiaohe"));
    assert!(!is_valid_id("pinyin"));
    assert!(!is_valid_id("none"));
    assert!(!is_valid_id("Huma"));
    assert!(!is_valid_id("hu+ma"));
}

#[test]
fn lists_bundled_schemes_first_then_user_schemes_by_id() {
    let root = scratch("list");
    let bundled = bundled(&root);
    let user = root.join("user");
    write(&user.join("schemes/zhengma/scheme.toml"), "");
    write(&user.join("schemes/zhengma/table.tsv"), "一\taa\t1\n");
    write(&user.join("schemes/bare/table.tsv"), "一\taa\t1\n");
    write(&user.join("schemes/odd/scheme.toml"), "type = \"stroke\"\n");
    write(&user.join("schemes/odd/table.tsv"), "一\taa\t1\n");
    write(
        &user.join("schemes/huma/scheme.toml"),
        "type = \"code\"\nname = \"虎码\"\nauthor = \"某人\"\nlicense = \"CC-BY-4.0\"\ntable = \"huma.tsv\"\nauto_commit = false\n",
    );
    write(&user.join("schemes/huma/huma.tsv"), "一\tyi\t1\n");
    // 码表不在的、目录名不合法的都跳过
    write(
        &user.join("schemes/broken/scheme.toml"),
        "name = \"坏的\"\n",
    );
    write(&user.join("schemes/xiaohe/scheme.toml"), "");
    write(&user.join("schemes/xiaohe/table.tsv"), "一\taa\t1\n");

    let entries = list(Some(&user), Some(&bundled));
    let ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["wubi86", "huma", "zhengma"]);
    assert_eq!(entries[0].label, "五笔（86）");
    assert!(entries[0].bundled && entries[0].auto_commit && entries[0].top);
    assert_eq!(entries[1].label, "虎码");
    assert_eq!(entries[1].meta.author, "某人");
    assert_eq!(entries[1].meta.license, "CC-BY-4.0");
    assert!(!entries[1].bundled && !entries[1].auto_commit && entries[1].top);
    assert_eq!(entries[2].label, "zhengma");
    assert!(entries[2].auto_commit && entries[2].top);
}

#[test]
fn a_user_scheme_with_a_bundled_id_takes_its_place() {
    let root = scratch("override");
    let bundled = bundled(&root);
    let user = root.join("user");
    write(&user.join("schemes/wubi86/scheme.toml"), "top = false\n");
    write(&user.join("schemes/wubi86/table.tsv"), "一\tggll\t1\n");

    let entries = list(Some(&user), Some(&bundled));
    assert_eq!(entries.len(), 1);
    assert!(!entries[0].bundled && !entries[0].top);
    assert_eq!(entries[0].label, "wubi86");
}

#[test]
fn the_old_user_wubi_table_still_wins_over_the_bundled_one() {
    let root = scratch("legacy");
    let bundled = bundled(&root);
    let user = root.join("user");
    write(&user.join("wubi/wubi86.tsv"), "一\tggll\t1\n");

    let wubi = find(WUBI86, Some(&user), Some(&bundled)).unwrap();
    assert_eq!(wubi.table, user.join("wubi/wubi86.tsv"));
    assert_eq!(wubi.label, "五笔（86）");
    assert!(find(WUBI86, Some(&user), None).is_none());
}

#[test]
fn loading_carries_the_rules_and_the_log_key() {
    let root = scratch("load");
    let user = root.join("user");
    write(
        &user.join("schemes/huma/scheme.toml"),
        "auto_commit = false\ncode_length = 3\n",
    );
    write(&user.join("schemes/huma/table.tsv"), "一\tyi\t1\n");
    let scheme = find("huma", Some(&user), None).unwrap().load().unwrap();
    assert_eq!(scheme.key, "huma");
    assert!(!scheme.auto_commit && scheme.top);
    assert_eq!(scheme.table.len(), 1);
    assert_eq!(scheme.code_length, 3);

    let bundled = bundled(&root);
    let wubi = find(WUBI86, None, Some(&bundled)).unwrap().load().unwrap();
    assert_eq!(wubi.key, "wubi");
    assert_eq!(wubi.code_length, 4);
}

/// 两条轴的配置：拼音侧写法与形码方案 id。
fn config(pinyin: &str, code: &str) -> Config {
    let mut config = Config::default();
    config.general.scheme = pinyin.to_owned();
    config.general.code = code.to_owned();
    config
}

#[test]
fn the_slot_reloads_only_when_the_scheme_or_its_table_changes() {
    let root = scratch("slot");
    let user = root.join("user");
    let table = user.join("schemes/huma/table.tsv");
    write(&user.join("schemes/huma/scheme.toml"), "");
    write(&table, "一\tyi\t1\n");
    let mut engine = Engine::new(Dictionary::parse("开\tkai\t100\n").unwrap());
    let mut slot = SchemeSlot::default();

    slot.apply(&mut engine, &config("none", "huma"), Some(&user), None);
    assert_eq!(slot.code_label(), Some("huma"));
    assert!(!engine.is_phonetic());
    // 把引擎里的方案悄悄换掉：再装一次同一套，没重读就还是换掉的那个
    let marker = CodeScheme::new("marker", CodeTable::parse("一\tyi\t1\n").unwrap());
    engine.set_code_scheme(Some(marker));
    slot.apply(&mut engine, &config("pinyin", "huma"), Some(&user), None);
    assert_eq!(engine.code_scheme().unwrap().key, "marker");
    assert!(engine.is_phonetic());

    write(&table, "一\tyi\t1\n二\ter\t1\n");
    slot.apply(&mut engine, &config("none", "huma"), Some(&user), None);
    assert_eq!(engine.code_scheme().unwrap().key, "huma");
    assert_eq!(engine.code_table().unwrap().len(), 2);

    slot.apply(&mut engine, &config("none", "off"), Some(&user), None);
    assert!(engine.code_scheme().is_none() && slot.code_label().is_none());
    assert!(engine.is_phonetic());

    slot.apply(&mut engine, &config("none", "missing"), Some(&user), None);
    assert!(engine.code_scheme().is_none() && engine.is_phonetic());
}

#[test]
fn rules_changed_in_the_config_apply_without_rereading_the_table() {
    let root = scratch("rules");
    let user = root.join("user");
    write(&user.join("schemes/huma/scheme.toml"), "top = false\n");
    write(&user.join("schemes/huma/table.tsv"), "一\tyi\t1\n");
    let mut engine = Engine::new(Dictionary::parse("开\tkai\t100\n").unwrap());
    let mut slot = SchemeSlot::default();
    let mut config = config("none", "huma");

    slot.apply(&mut engine, &config, Some(&user), None);
    let scheme = engine.code_scheme().unwrap();
    assert!(scheme.auto_commit && !scheme.top);

    let marker = CodeScheme::new("marker", CodeTable::parse("一\tyi\t1\n").unwrap());
    engine.set_code_scheme(Some(marker));
    config.code.insert(
        "huma".to_owned(),
        CodeRules {
            auto_commit: Some(false),
            top: Some(true),
        },
    );
    slot.apply(&mut engine, &config, Some(&user), None);
    let scheme = engine.code_scheme().unwrap();
    assert_eq!(scheme.key, "marker");
    assert!(!scheme.auto_commit && scheme.top);
}

#[test]
fn imported_code_words_join_their_own_scheme() {
    let root = scratch("words");
    let bundled = bundled(&root);
    let user = root.join("user");
    write(&user.join("schemes/huma/scheme.toml"), "");
    write(&user.join("schemes/huma/table.tsv"), "一\tyi\t1\n");
    let words = user.join("code_dicts/mine.tsv");
    write(&words, "# scheme: wubi86\n青简\tgetu\t1\n");
    write(
        &user.join("code_dicts/huma-words.tsv"),
        "# scheme: huma\n青简\tqj\t1\n",
    );
    let mut engine = Engine::new(Dictionary::parse("开\tkai\t100\n").unwrap());
    let mut slot = SchemeSlot::default();
    let mut config = config("pinyin", "wubi86");

    slot.apply(&mut engine, &config, Some(&user), Some(&bundled));
    let table = engine.code_table().unwrap();
    assert_eq!(table.len(), 2);
    assert_eq!(table.lookup("getu", 10)[0].text, "青简");
    assert!(table.lookup("qj", 10).is_empty());

    // 词库改了、关掉了都要重装，不能当成同一张码表
    write(&words, "# scheme: wubi86\n青简\tgetu\t1\n小青\tihge\t1\n");
    slot.apply(&mut engine, &config, Some(&user), Some(&bundled));
    assert_eq!(engine.code_table().unwrap().len(), 3);
    config.dictionaries.disabled = vec!["mine".to_owned()];
    slot.apply(&mut engine, &config, Some(&user), Some(&bundled));
    assert_eq!(engine.code_table().unwrap().len(), 1);
}
