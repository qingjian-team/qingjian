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

#[test]
fn exported_phrases_import_back_and_configs_are_refused() {
    let phrases = vec![
        CustomPhrase {
            weight: 30,
            enabled: false,
            ..CustomPhrase::new("dz", "第一行\n第二行 \"引号\" $(Year)")
        },
        CustomPhrase::new("ee", "；"),
    ];
    let exported = qingjian_platform::export_phrases(&phrases);
    assert_eq!(
        qingjian_platform::import_phrases(&exported).unwrap(),
        (phrases, Vec::new())
    );
    let bad = "[[custom_phrases]]\ncode = 'ee'\ntext = '：'\n\n\
               [[custom_phrases]]\ncode = 'A1'\ntext = '写错的'\n\n\
               [[custom_phrases]]\ncode = 'ee'\ntext = '：'\n";
    // 别的测试读配置时会改界面语言，比对文案要固定下来
    bamboo_slip_i18n::pin_locale("zh-Hans");
    let (read, skipped) = qingjian_platform::import_phrases(bad).unwrap();
    assert_eq!(read, vec![CustomPhrase::new("ee", "：")]);
    assert_eq!(skipped.len(), 2);
    assert!(
        skipped[0].starts_with("第 2 条：输入码须为"),
        "{}",
        skipped[0]
    );
    assert!(skipped[1].starts_with("第 3 条："), "{}", skipped[1]);
    // 设置与短语各自导入：整份配置文件不收
    let config = format!("[general]\npage_size = 5\n\n{bad}");
    assert!(
        qingjian_platform::import_phrases(&config)
            .unwrap_err()
            .contains("配置文件")
    );
    assert!(qingjian_platform::import_phrases("[general]\n").is_err());
    assert!(qingjian_platform::import_phrases("").is_err());
}

#[test]
fn phrase_files_saved_with_a_bom_import_clean() {
    // 带 BOM 的短语文件：第一条的文字不能带上看不见的 U+FEFF，否则上屏时一起打进去
    let (read, _) = qingjian_platform::import_phrases("\u{feff}你好\tnh\t5\n").unwrap();
    assert_eq!(read[0].text, "你好");
    let exported = qingjian_platform::export_phrases(&[CustomPhrase::new("nh", "你好")]);
    let (read, _) = qingjian_platform::import_phrases(&format!("\u{feff}{exported}")).unwrap();
    assert_eq!(read, vec![CustomPhrase::new("nh", "你好")]);
}

/// Rime `custom_phrase.txt`：文字<Tab>输入码<Tab>权重，文件头与注释跳过，权重夹到 1–999。
#[test]
fn rime_style_tsv_imports() {
    let source = "# Rime table\n---\nname: custom_phrase\n...\n\n地址\tdz\t1\n青简输入法\tqj\t5000\n没码\n你好\tnh\n⑪\tyq11\n";
    bamboo_slip_i18n::pin_locale("zh-Hans");
    let (read, skipped) = qingjian_platform::import_phrases(source).unwrap();
    assert_eq!(
        read,
        vec![
            CustomPhrase {
                weight: 1,
                ..CustomPhrase::new("dz", "地址")
            },
            CustomPhrase {
                weight: 999,
                ..CustomPhrase::new("qj", "青简输入法")
            },
            CustomPhrase::new("nh", "你好"),
        ]
    );
    // #857：码里带数字的序号符号要说清是哪一行、为什么
    assert_eq!(skipped.len(), 2);
    assert_eq!(skipped[0], "第 8 行「没码」：文字与输入码之间要用 Tab 隔开");
    assert!(
        skipped[1].starts_with("第 10 行「⑪」：输入码须为 1–") && skipped[1].contains("不能带数字"),
        "{}",
        skipped[1]
    );
}
