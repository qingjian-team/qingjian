use qingjian_core::CustomPhrase;
use qingjian_platform::Config;

#[test]
fn save_roundtrip_preserves_text_and_rejects_conflicting_positions() {
    let dir = std::env::temp_dir().join(format!(
        "qingjian-phrases-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    std::fs::write(
        &path,
        "# existing comment\n[general]\npage_size = 5\nfull_width_punctuation = false\n",
    )
    .unwrap();
    let phrases = vec![
        CustomPhrase {
            code: "prompt".into(),
            text: "  before\nlong text\n ".repeat(4000),
            position: 1,
            enabled: false,
        },
        CustomPhrase {
            code: "ee".into(),
            text: "；".into(),
            position: 2,
            enabled: true,
        },
    ];
    Config::set_custom_phrases(&path, &phrases).unwrap();
    let parsed = Config::load(&path).unwrap();
    assert_eq!(parsed.custom_phrases, phrases);
    assert!(!parsed.general.full_width_punctuation);
    assert_eq!(parsed.general.page_size(), 5);
    let before = std::fs::read_to_string(&path).unwrap();
    assert!(before.contains("# existing comment"));
    let mut invalid = phrases.clone();
    invalid.push(phrases[1].clone());
    let error = Config::set_custom_phrases(&path, &invalid).unwrap_err();
    assert!(error.contains("ee") && error.contains('2'));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
    std::fs::write(
        &path,
        format!("{before}\n[[custom_phrases]]\ncode = 'ee'\ntext = '：'\nposition = 2\n"),
    )
    .unwrap();
    // 与已有 `ee` 第 2 位冲突：写入被拒，但读回来时那条**被丢掉**而不是整份配置失效
    let (config, dropped) = Config::load_with_diagnostics(&path).unwrap();
    assert_eq!(config.custom_phrases, phrases);
    assert!(!dropped.is_empty());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn custom_log_source_preserves_existing_shortcut_encoding() {
    use qingjian_core::InputSource;
    assert_eq!(
        serde_json::to_string(&InputSource::Custom).unwrap(),
        "\"custom\""
    );
    assert_eq!(
        serde_json::from_str::<InputSource>("\"custom\"").unwrap(),
        InputSource::Custom
    );
    assert_eq!(
        serde_json::from_str::<InputSource>("\"shortcut\"").unwrap(),
        InputSource::Shortcut
    );
}

#[test]
fn phrase_comments_follow_edits_reordering_and_deletion() {
    let dir = std::env::temp_dir().join(format!("qingjian-comments-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    std::fs::write(
        &path,
        r#"# global
[general]
page_size = 5 # page
[[custom_phrases]] # first
code = "aa" # code
text = "甲" # text
position = 1 # position
enabled = true # enabled
[[custom_phrases]] # second
code = "bb"
text = "乙"
position = 2
enabled = true
"#,
    )
    .unwrap();
    let mut phrases = Config::load(&path).unwrap().custom_phrases;
    phrases[0].enabled = false;
    phrases[0].text = "新内容\n第二行".into();
    phrases[0].code = "cc".into();
    phrases[0].position = 3;
    Config::set_custom_phrases(&path, &phrases).unwrap();
    let saved = std::fs::read_to_string(&path).unwrap();
    for comment in [
        "# global",
        "# page",
        "# first",
        "# code",
        "# text",
        "# position",
        "# enabled",
        "# second",
    ] {
        assert!(saved.contains(comment), "{comment}");
    }
    assert_eq!(Config::load(&path).unwrap().custom_phrases, phrases);
    phrases.swap(0, 1);
    Config::set_custom_phrases(&path, &phrases).unwrap();
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(saved.find("# second").unwrap() < saved.find("# first").unwrap());
    phrases.remove(0);
    Config::set_custom_phrases(&path, &phrases).unwrap();
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(saved.contains("# first") && saved.contains("# text"));
    assert!(!saved.contains("# second"));
    phrases.clear();
    Config::set_custom_phrases(&path, &phrases).unwrap();
    assert!(Config::load(&path).unwrap().custom_phrases.is_empty());
    std::fs::remove_dir_all(dir).unwrap();
}
