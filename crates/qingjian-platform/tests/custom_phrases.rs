use qingjian_core::CustomPhrase;
use qingjian_platform::Config;

#[test]
fn save_roundtrip_preserves_text_and_rejects_duplicate_phrases() {
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
            enabled: false,
            ..CustomPhrase::new("prompt", &"  before\nlong text\n ".repeat(4000))
        },
        CustomPhrase {
            weight: 3,
            ..CustomPhrase::new("ee", "；")
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
    assert!(error.contains("ee"));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
    std::fs::write(
        &path,
        format!("{before}\n[[custom_phrases]]\ncode = 'ee'\ntext = '；'\nweight = 20\n"),
    )
    .unwrap();
    // 手改出来的冲突条目只丢它自己，前面的短语和其余设置照常生效
    let loaded = Config::load(&path).unwrap();
    assert_eq!(loaded.custom_phrases, phrases);
    assert_eq!(loaded.general.page_size(), 5);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn bad_phrases_are_skipped_on_load_and_kept_on_save() {
    let dir = std::env::temp_dir().join(format!(
        "qingjian-phrases-bad-{}-{}",
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
        "[general]\nscheme = \"xiaohe\"\n\n\
         [[custom_phrases]]\ncode = \"ww\"\ntext = \"；\"\nposition = 1\n\n\
         [[custom_phrases]] # 越界\ncode = \"ddz\"\ntext = \"等等\"\nposition = 10\n\n\
         [[custom_phrases]]\ncode = \"yx\"\ntext = 42\nposition = 1\n\n\
         [[custom_phrases]]\ncode = \"dz\"\ntext = \"地址\"\nposition = 2\n",
    )
    .unwrap();
    let loaded = Config::load(&path).unwrap();
    assert_eq!(loaded.general.scheme, "xiaohe");
    let codes: Vec<_> = loaded
        .custom_phrases
        .iter()
        .map(|p| p.code.as_str())
        .collect();
    assert_eq!(codes, ["ww", "dz"]);

    // 偏好设置里停用一条再保存：跳过的两条原样留在文件里
    let mut phrases = loaded.custom_phrases;
    phrases[0].enabled = false;
    Config::set_custom_phrases(&path, &phrases).unwrap();
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(saved.contains("[[custom_phrases]] # 越界"));
    assert!(saved.contains("code = \"ddz\"") && saved.contains("position = 10"));
    assert!(saved.contains("code = \"yx\"") && saved.contains("text = 42"));
    let reloaded = Config::load(&path).unwrap();
    assert_eq!(reloaded.custom_phrases, phrases);
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
weight = 10 # weight
enabled = true # enabled
[[custom_phrases]] # second
code = "bb"
text = "乙"
weight = 10
enabled = true
"#,
    )
    .unwrap();
    let mut phrases = Config::load(&path).unwrap().custom_phrases;
    phrases[0].enabled = false;
    phrases[0].text = "新内容\n第二行".into();
    phrases[0].code = "cc".into();
    phrases[0].weight = 3;
    Config::set_custom_phrases(&path, &phrases).unwrap();
    let saved = std::fs::read_to_string(&path).unwrap();
    for comment in [
        "# global",
        "# page",
        "# first",
        "# code",
        "# text",
        "# weight",
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

#[test]
fn old_positions_are_saved_as_weights_in_place() {
    let dir = std::env::temp_dir().join(format!("qingjian-weights-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    std::fs::write(
        &path,
        "[[custom_phrases]]\ncode = \"ee\"\ntext = \"：\"\nposition = 1 # 首位\nenabled = true\n\n\
         [[custom_phrases]]\ncode = \"ee\"\ntext = \"；\"\nposition = 2\nenabled = true\n",
    )
    .unwrap();
    let mut phrases = Config::load(&path).unwrap().custom_phrases;
    assert_eq!(
        phrases.iter().map(|p| p.weight).collect::<Vec<_>>(),
        [10, 9]
    );
    phrases[1].enabled = false;
    Config::set_custom_phrases(&path, &phrases).unwrap();
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(!saved.contains("position"));
    assert!(saved.contains("text = \"：\"\nweight = 10 # 首位\nenabled = true"));
    assert!(saved.contains("text = \"；\"\nweight = 9\nenabled = false"));
    assert_eq!(Config::load(&path).unwrap().custom_phrases, phrases);
    std::fs::remove_dir_all(dir).unwrap();
}
