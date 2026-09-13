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
    assert!(Config::load(&path).is_err());
    std::fs::remove_dir_all(dir).unwrap();
}
