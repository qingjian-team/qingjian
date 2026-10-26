use std::path::{Path, PathBuf};

use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;

use super::*;
use crate::{Config, Scheme, SchemeSlot};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qingjian-shuangpin-schemes-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// 小鹤当底子，把 ing 挪到 `;`、uai 留在 k。
const MINE: &str = r#"
type = "shuangpin"
name = "我的小鹤"
base = "xiaohe"

[finals]
k = "uai"
";" = "ing"
"#;

#[test]
fn only_folders_marked_shuangpin_are_listed() {
    let root = scratch("list");
    let user = root.join("user");
    write(&user.join("schemes/mine/scheme.toml"), MINE);
    write(&user.join("schemes/huma/scheme.toml"), "name = \"虎码\"\n");
    write(&user.join("schemes/huma/table.tsv"), "一\tyi\t1\n");
    write(
        &user.join("schemes/broken/scheme.toml"),
        "type = \"shuangpin\"\nbase = \"flypy\"\n",
    );
    write(&user.join("schemes/xiaohe/scheme.toml"), MINE);

    let entries = list(Some(&user), None);
    let ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["mine"]);
    assert_eq!(entries[0].label, "我的小鹤");
    let codes: Vec<String> = crate::code_schemes::list(Some(&user), None)
        .into_iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(codes, ["huma"]);
}

#[test]
fn a_scheme_overrides_its_base_key_by_key() {
    let root = scratch("override");
    let user = root.join("user");
    write(&user.join("schemes/mine/scheme.toml"), MINE);
    let layout = find("mine", Some(&user), None).unwrap().layout;
    assert_eq!(layout.key(), "mine");
    assert!(layout.uses_semicolon());
    assert_eq!(layout.decode("x;").pinyin(), "xing");
    assert_eq!(layout.decode("kk").pinyin(), "kuai");
    assert_eq!(layout.decode("nihc").pinyin(), "ni'hao");
    // 两键配不出音节时按每键一个声母的简拼读
    assert_eq!(layout.decode("xk").pinyin(), "x'k");
}

#[test]
fn a_scheme_without_a_base_is_written_from_scratch() {
    let root = scratch("scratch");
    let user = root.join("user");
    write(
        &user.join("schemes/tiny/scheme.toml"),
        r#"
type = "shuangpin"
[initials]
v = "zh"
[finals]
a = "a"
i = "i"
s = ["iong", "ong"]
[zero]
a = "aa"
"#,
    );
    let entry = find("tiny", Some(&user), None).unwrap();
    assert_eq!(entry.label, "tiny");
    assert_eq!(entry.layout.decode("vsaa").pinyin(), "zhong'a");
    assert_eq!(entry.layout.decode("ha").pinyin(), "ha");
    // 两键配不出音节时按每键一个声母的简拼读
    assert_eq!(entry.layout.decode("hk").pinyin(), "h'k");
}

#[test]
fn the_slot_loads_a_custom_layout_and_falls_back_to_full_pinyin() {
    let root = scratch("slot");
    let user = root.join("user");
    write(&user.join("schemes/mine/scheme.toml"), MINE);
    let mut engine = Engine::new(Dictionary::parse("星\txing\t100\n").unwrap());
    let mut slot = SchemeSlot::default();

    let mut config = Config::default();
    config.general.scheme = "mine".to_owned();
    slot.apply(&mut engine, &config, Some(&user), None);
    assert_eq!(engine.shuangpin().unwrap().key(), "mine");
    assert_eq!(slot.scheme_label(Scheme::Custom), "我的小鹤");

    config.general.scheme = "xiaohe".to_owned();
    slot.apply(&mut engine, &config, Some(&user), None);
    assert_eq!(engine.shuangpin().unwrap().key(), "xiaohe");
    assert_eq!(slot.shuangpin_label(), None);

    config.general.scheme = "gone".to_owned();
    slot.apply(&mut engine, &config, Some(&user), None);
    assert!(engine.shuangpin().is_none());
    assert_eq!(slot.scheme_name(Scheme::Custom), "全拼");
}
