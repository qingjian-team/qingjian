//! 码表与导入词库变化时清理旧输入和展示身份。

use super::{PURE, open_and_type, scheme, texts};
use crate::dispatch::reload::tests::{poll, temp_dir, watched};
use qingjian_platform::protocol::SessionId;

#[test]
fn selected_table_change_cancels_input_and_removal_falls_back() {
    let dir = temp_dir("code-table-change");
    scheme(&dir, true, "开\tga\t10\n");
    let mut router = watched(&dir, PURE);
    open_and_type(&mut router, 1, "g");
    let path = dir.join("data/generated/schemes/wubi86/table.tsv");
    std::fs::write(&path, "改\tga\t100\n").unwrap();
    poll(&mut router);
    assert!(router.engine.composition().is_empty());
    assert_eq!(texts(&mut router, "ga"), ["改"]);
    std::fs::remove_file(path).unwrap();
    poll(&mut router);
    assert!(router.engine.composition().is_empty());
    assert!(router.engine.code_scheme().is_none());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn unrelated_scheme_changes_keep_composition_and_display_identity() {
    let dir = temp_dir("code-unrelated");
    scheme(&dir, true, "开\tga\t10\n");
    let mut router = watched(&dir, PURE);
    open_and_type(&mut router, 1, "g");
    let session = SessionId(1);
    router.handle_linux(serde_json::json!({"DisplayReporting": {
        "session": session,
        "identity": {"generation": 1, "context": "code-test", "revision": 0}
    }}));
    let identity = router.sessions[&session].display_identity.clone();
    let path = dir.join("custom/schemes/other");
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(path.join("scheme.toml"), "name = \"另一个方案\"\n").unwrap();
    std::fs::write(path.join("table.tsv"), "另\taa\t10\n").unwrap();
    poll(&mut router);
    assert_eq!(router.engine.composition().text(), "g");
    assert_eq!(router.sessions[&session].display_identity, identity);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn packed_bundled_wubi_is_loaded_and_reloaded() {
    let dir = temp_dir("packed-wubi");
    let bundled = dir.join("data/generated/schemes");
    std::fs::create_dir_all(&bundled).unwrap();
    let table = bundled.join("wubi86.qj");
    qingjian_dictionary::CodeTable::parse("甲\tabcd\t10\n乙\tefgh\t10\n")
        .unwrap()
        .write_qj(&table, &Default::default())
        .unwrap();
    let mut router = watched(&dir, PURE);
    assert_eq!(texts(&mut router, "abcd"), ["甲"]);
    assert_eq!(
        router
            .engine
            .code_scheme()
            .unwrap()
            .phrase_code("甲乙")
            .as_deref(),
        Some("abef")
    );
    qingjian_dictionary::CodeTable::parse("修改后\tabcd\t100\n")
        .unwrap()
        .write_qj(&table, &Default::default())
        .unwrap();
    poll(&mut router);
    assert_eq!(texts(&mut router, "abcd"), ["修改后"]);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn code_words_edits_and_deletions_invalidate_previous_candidates() {
    let dir = temp_dir("code-words-edit");
    scheme(&dir, true, "开\tga\t10\n");
    let path = dir.join("custom/code_dicts/extra.tsv");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "# scheme: wubi86\n新增\tga\t20\n").unwrap();
    let mut router = watched(&dir, PURE);
    open_and_type(&mut router, 1, "g");
    let session = SessionId(1);
    router.handle_linux(serde_json::json!({"DisplayReporting": {
        "session": session,
        "identity": {"generation": 1, "context": "code-test", "revision": 0}
    }}));
    let identity = router.sessions[&session].display_identity.clone();
    assert!(router.valid_panel_event(session, identity.as_ref().unwrap()));
    std::fs::write(&path, "# scheme: wubi86\n修改后\tga\t30\n").unwrap();
    poll(&mut router);
    assert!(router.engine.composition().is_empty());
    assert_ne!(router.sessions[&session].display_identity, identity);
    assert!(!router.valid_panel_event(session, identity.as_ref().unwrap()));
    assert!(router.sessions[&session].display_frame.is_none());
    assert!(router.sessions[&session].last_frame.is_none());
    let candidates = texts(&mut router, "ga");
    assert!(candidates.contains(&"修改后".to_owned()));
    assert!(!candidates.contains(&"新增".to_owned()));
    std::fs::remove_file(path).unwrap();
    poll(&mut router);
    assert_eq!(texts(&mut router, "ga"), ["开"]);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn custom_table_path_is_watched_without_descriptor_changes() {
    let dir = temp_dir("code-nested-table");
    let scheme_dir = dir.join("custom/schemes/custom");
    std::fs::create_dir_all(scheme_dir.join("tables")).unwrap();
    std::fs::write(
        scheme_dir.join("scheme.toml"),
        "table = \"tables/words.tsv\"\n",
    )
    .unwrap();
    let table = scheme_dir.join("tables/words.tsv");
    std::fs::write(&table, "开\tga\t10\n").unwrap();
    let mut router = watched(&dir, "[general]\nscheme = \"none\"\ncode = \"custom\"\n");
    assert_eq!(texts(&mut router, "ga"), ["开"]);
    std::fs::write(&table, "修改\tga\t100\n").unwrap();
    poll(&mut router);
    assert_eq!(texts(&mut router, "ga"), ["修改"]);
    std::fs::remove_file(&table).unwrap();
    poll(&mut router);
    assert!(router.engine.code_scheme().is_none());
    std::fs::write(table, "恢复\tga\t1000\n").unwrap();
    poll(&mut router);
    assert_eq!(texts(&mut router, "ga"), ["恢复"]);
    std::fs::remove_dir_all(dir).unwrap();
}
