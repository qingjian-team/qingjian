use std::path::{Path, PathBuf};

use super::*;
use crate::AuxCodeConfig;
use crate::code_tables::AuxCodeSources;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qingjian-aux-schemes-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

const SHOUYOU: &str = r#"
type = "aux"
name = "首右"

[words]
2 = "AaBa"
3 = "AaBaZa"
"#;

#[test]
fn only_folders_marked_aux_are_listed() {
    let user = scratch("list");
    write(&user.join("schemes/shouyou/scheme.toml"), SHOUYOU);
    write(&user.join("schemes/shouyou/table.tsv"), "开\tkg\n");
    write(&user.join("schemes/huma/scheme.toml"), "name = \"虎码\"\n");
    write(&user.join("schemes/huma/table.tsv"), "一\tyi\t1\n");
    write(
        &user.join("schemes/no-table/scheme.toml"),
        "type = \"aux\"\n",
    );
    write(
        &user.join("schemes/bad-rule/scheme.toml"),
        "type = \"aux\"\n[words]\n2 = \"aA\"\n",
    );
    write(&user.join("schemes/bad-rule/table.tsv"), "开\tkg\n");

    let entries = list(Some(&user), None);
    let ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["shouyou"]);
    assert_eq!(entries[0].label, "首右");
    let codes: Vec<String> = crate::code_schemes::list(Some(&user), None)
        .into_iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(codes, ["huma"]);
}

#[test]
fn only_the_chosen_set_is_loaded() {
    let user = scratch("load");
    write(&user.join("schemes/shouyou/scheme.toml"), SHOUYOU);
    write(&user.join("schemes/shouyou/table.tsv"), "开\tkg\n发\tfy\n");
    write(
        &user.join("schemes/stroke-x/scheme.toml"),
        "type = \"aux\"\n",
    );
    write(&user.join("schemes/stroke-x/table.tsv"), "开\thh\n");
    let sources = AuxCodeSources {
        user_dir: Some(user.clone()),
        ..AuxCodeSources::default()
    };
    let first_code = |config: &AuxCodeConfig| {
        let tables = sources.load(config);
        assert!(tables.len() <= 1);
        tables.first().and_then(|table| {
            table
                .code_with_prefix("开", "")
                .map(|code| code.into_owned())
        })
    };

    assert_eq!(first_code(&AuxCodeConfig::default()).as_deref(), Some("kg"));
    let chosen = AuxCodeConfig {
        scheme: "stroke-x".to_owned(),
        ..AuxCodeConfig::default()
    };
    assert_eq!(first_code(&chosen).as_deref(), Some("hh"));
    // 选的不在了，退回第一套没被关掉的
    let stale = AuxCodeConfig {
        scheme: "gone".to_owned(),
        disabled: vec!["shouyou".to_owned()],
        ..AuxCodeConfig::default()
    };
    assert_eq!(first_code(&stale).as_deref(), Some("hh"));
    let words = sources.load(&AuxCodeConfig::default());
    assert_eq!(words[0].code_with_prefix("开发", "").as_deref(), Some("kf"));
}

#[test]
fn the_snapshot_sees_files_inside_scheme_folders() {
    let user = scratch("snapshot");
    let sources = AuxCodeSources {
        user_dir: Some(user.clone()),
        ..AuxCodeSources::default()
    };
    let before = sources.snapshot();
    write(&user.join("schemes/shouyou/scheme.toml"), SHOUYOU);
    write(&user.join("schemes/shouyou/table.tsv"), "开\tkg\n");
    let after = sources.snapshot();
    assert!(before.is_empty());
    assert_eq!(after.len(), 2);
}

#[test]
fn the_bundled_stroke_table_derives_words_it_does_not_list() {
    let root = scratch("stroke");
    let bundled = root.join("codes");
    std::fs::create_dir_all(&bundled).unwrap();
    let table =
        qingjian_dictionary::AuxCodeTable::parse("开\thpsnz\n发\tzpnn\n展\tzhpn\n开发\thz\n")
            .unwrap();
    table
        .write_qj(&bundled.join("stroke.qj"), &Default::default())
        .unwrap();
    let sources = AuxCodeSources {
        bundled_codes: Some(bundled),
        ..AuxCodeSources::default()
    };

    let tables = sources.load(&AuxCodeConfig::default());
    assert_eq!(
        tables[0].code_with_prefix("开发", "").as_deref(),
        Some("hz")
    );
    assert_eq!(
        tables[0].code_with_prefix("开展", "").as_deref(),
        Some("hz")
    );
    assert_eq!(
        tables[0].code_with_prefix("开发展开", "").as_deref(),
        Some("hzzh")
    );
}
