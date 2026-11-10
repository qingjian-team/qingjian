use super::*;

fn temp(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("qingjian-user-dirs-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// 旧版本平铺的文件逐个归进分类文件夹，配置与剪贴板留在根下。
#[test]
fn flat_layout_is_sorted_into_folders() {
    let root = temp("flat");
    for name in [
        "config.toml",
        "user.tsv",
        "user-words.tsv",
        "user-code-words.tsv",
        "user-code-words.tsv.bak",
        "user-english.tsv.bak",
        "user-glossary-ja.tsv",
        "user-vocab.tsv",
        "usage-hourly.tsv",
        "input-log.jsonl",
        "update.json",
        "themes/fluent/theme.json",
        "schemes/wubi/scheme.toml",
        "models/hanzhang-tongbian/model.qjm",
        "glossaries/glossary-ja.qj",
        "clipboard/history.db",
    ] {
        write(&root.join(name), name);
    }
    write(
        &root.join("dict-index.json"),
        r#"{"format":1,"dicts":[{"id":"domain-law"}],"bundled":[{"id":"base"}]}"#,
    );
    write(&root.join("dicts/domain-law.qj"), "downloaded");
    write(&root.join("dicts/my-words.qj"), "imported");
    write(&root.join("dicts/removed/old.qj"), "removed");

    let dirs = UserDirs::at(root.clone());
    dirs.prepare();

    for (old, new) in [
        ("user.tsv", "learning/user.tsv"),
        ("user-words.tsv", "learning/user-words.tsv"),
        ("user-code-words.tsv", "learning/user-code-words.tsv"),
        (
            "user-code-words.tsv.bak",
            "learning/user-code-words.tsv.bak",
        ),
        ("user-english.tsv.bak", "learning/user-english.tsv.bak"),
        ("user-glossary-ja.tsv", "learning/user-glossary-ja.tsv"),
        ("user-vocab.tsv", "records/user-vocab.tsv"),
        ("usage-hourly.tsv", "records/usage-hourly.tsv"),
        ("input-log.jsonl", "records/input-log.jsonl"),
        ("update.json", "cache/update.json"),
        ("dict-index.json", "cache/dict-index.json"),
        ("themes", "custom/themes/fluent/theme.json"),
        ("schemes", "custom/schemes/wubi/scheme.toml"),
        ("models", "downloads/models/hanzhang-tongbian/model.qjm"),
        ("glossaries", "downloads/glossaries/glossary-ja.qj"),
        ("dicts", "downloads/dicts/domain-law.qj"),
        ("dicts", "custom/dicts/my-words.qj"),
        ("dicts", "custom/dicts/removed/old.qj"),
    ] {
        assert!(!root.join(old).exists(), "{old} 还在根下");
        assert!(root.join(new).is_file(), "{new} 没搬到");
    }
    assert!(root.join("config.toml").is_file());
    assert!(root.join("clipboard/history.db").is_file());
    let _ = std::fs::remove_dir_all(&root);
}

/// 新位置已经有同名的就不覆盖，旧的留着；再跑一次也不出错。
#[test]
fn existing_targets_are_kept() {
    let root = temp("existing");
    write(&root.join("user.tsv"), "old");
    write(&root.join("learning/user.tsv"), "new");
    let dirs = UserDirs::at(root.clone());
    dirs.prepare();
    dirs.prepare();
    assert_eq!(
        std::fs::read_to_string(root.join("learning/user.tsv")).unwrap(),
        "new"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("user.tsv")).unwrap(),
        "old"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// 下载与缓存放在另一处（Windows 的 `%LOCALAPPDATA%`）时搬过去，用户产生的仍在根下。
#[test]
fn downloads_and_cache_go_to_local() {
    let root = temp("roaming");
    let local = temp("local");
    write(&root.join("models/zhiyi/model.bin"), "model");
    write(&root.join("update.json"), "{}");
    write(&root.join("user.tsv"), "words");
    UserDirs::new(root.clone(), local.clone()).prepare();
    assert!(local.join("downloads/models/zhiyi/model.bin").is_file());
    assert!(local.join("cache/update.json").is_file());
    assert!(root.join("learning/user.tsv").is_file());
    assert!(!root.join("models").exists());
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&local);
}

/// 新装的（根目录不在或是空的）记下没有要补下的词库；从老版本升上来的不写，留给设置程序去算。
#[test]
fn only_fresh_installs_get_an_empty_restore_list() {
    let fresh = temp("fresh");
    let dirs = UserDirs::at(fresh.clone());
    dirs.prepare();
    assert_eq!(std::fs::read_to_string(dirs.restore_list()).unwrap(), "");

    let upgraded = temp("upgraded");
    write(&upgraded.join("learning/user.tsv"), "words");
    let dirs = UserDirs::at(upgraded.clone());
    dirs.prepare();
    assert!(!dirs.restore_list().exists());
    dirs.prepare();
    assert!(!dirs.restore_list().exists());
    let _ = std::fs::remove_dir_all(&fresh);
    let _ = std::fs::remove_dir_all(&upgraded);
}
