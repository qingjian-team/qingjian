use super::*;

#[test]
fn imports_tsv_and_rime_into_qj() {
    let dir = std::env::temp_dir().join(format!("qingjian-import-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let tsv = dir.join("finance.tsv");
    std::fs::write(&tsv, "账套\tzhang tao\t500\n").unwrap();
    let imported = import(&tsv, &dir.join("dicts")).unwrap();
    assert_eq!(imported.name, "finance");
    assert_eq!(imported.entries, 1);
    let dictionary = Dictionary::from_path(&imported.path).unwrap();
    assert_eq!(dictionary.lookup(&["zhang", "tao"], false)[0].text, "账套");
    assert_eq!(dictionary.metadata().unwrap().name, "finance");

    let rime = dir.join("law.dict.yaml");
    std::fs::write(
        &rime,
        "# Rime dictionary\n---\nname: law\nversion: \"1\"\nsort: by_weight\n...\n合同法\the tong fa\t120\n民法典\tmin fa dian\n",
    )
    .unwrap();
    let imported = import(&rime, &dir.join("dicts")).unwrap();
    assert_eq!(imported.path.file_name().unwrap(), "law.qj");
    assert_eq!(imported.name, "law");
    assert_eq!(imported.entries, 2);
    let dictionary = Dictionary::from_path(&imported.path).unwrap();
    assert_eq!(
        dictionary.lookup(&["min", "fa", "dian"], false)[0].text,
        "民法典"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn imports_files_saved_with_a_bom() {
    // 记事本、Excel 存的「UTF-8 带 BOM」：Rime 词库要认得出 YAML 头，TSV 的第一个词不能带上看不见的 U+FEFF
    let dir = std::env::temp_dir().join(format!("qingjian-import-bom-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let tsv = dir.join("finance.tsv");
    std::fs::write(&tsv, "\u{feff}账套\tzhang tao\t500\n").unwrap();
    let imported = import(&tsv, &dir.join("dicts")).unwrap();
    let dictionary = Dictionary::from_path(&imported.path).unwrap();
    assert_eq!(dictionary.lookup(&["zhang", "tao"], false)[0].text, "账套");

    let rime = dir.join("law.dict.yaml");
    std::fs::write(
        &rime,
        "\u{feff}# Rime dictionary\n---\nname: law\n...\n民法典\tmin fa dian\n",
    )
    .unwrap();
    let imported = import(&rime, &dir.join("dicts")).unwrap();
    assert_eq!(imported.name, "law");
    let dictionary = Dictionary::from_path(&imported.path).unwrap();
    assert_eq!(
        dictionary.lookup(&["min", "fa", "dian"], false)[0].text,
        "民法典"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn imports_tone_numbered_pinyin() {
    let dir = std::env::temp_dir().join(format!("qingjian-import-tones-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let tsv = dir.join("toned.tsv");
    std::fs::write(&tsv, "东西\tdong1 xi5\t20\n绿色\tlv4 se4\t10\n").unwrap();
    assert!(!format::is_code_table(
        &std::fs::read_to_string(&tsv).unwrap()
    ));
    let imported = import(&tsv, &dir.join("dicts")).unwrap();
    let dictionary = Dictionary::from_path(&imported.path).unwrap();
    let hit = dictionary.lookup(&["dong", "xi"], false)[0];
    assert_eq!((hit.text, hit.pinyin), ("东西", "dong xi"));
    assert_eq!(hit.tones.to_vec(2), [1, 5]);
    assert_eq!(dictionary.lookup(&["lv", "se"], false)[0].text, "绿色");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn refuses_wubi_words_as_pinyin() {
    let dir = std::env::temp_dir().join(format!("qingjian-import-wubi-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let tsv = dir.join("wubi-test.tsv");
    std::fs::write(&tsv, "拉混\truji\t1\n青简\tgetu\t1\n").unwrap();
    assert!(matches!(
        import(&tsv, &dir.join("dicts")),
        Err(DictionaryError::CodeWords)
    ));
    assert!(!dir.join("dicts/wubi-test.qj").exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn imports_word_list_deriving_readings_from_the_reference() {
    let dir = std::env::temp_dir().join(format!("qingjian-import-words-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let reference = Dictionary::parse("李\tli\t900\n三\tsan\t900\n").unwrap();
    let list = dir.join("names.txt");
    std::fs::write(&list, "李三\n囧三\n").unwrap();

    let imported = import_word_list(&list, &dir.join("dicts"), &[&reference]).unwrap();
    assert_eq!(imported.imported.path.file_name().unwrap(), "names.qj");
    assert_eq!(imported.imported.name, "names");
    assert_eq!(imported.imported.entries, 1);
    assert_eq!(imported.report.words, 1);
    assert_eq!(imported.report.unknown, ["囧三".to_owned()]);
    let dictionary = Dictionary::from_path(&imported.imported.path).unwrap();
    assert_eq!(dictionary.lookup(&["li", "san"], false)[0].text, "李三");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn rejects_word_list_without_any_derivable_word() {
    let dir = std::env::temp_dir().join(format!("qingjian-import-names-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let reference = Dictionary::parse("三\tsan\t900\n").unwrap();
    let list = dir.join("names.txt");
    std::fs::write(&list, "囧囧\n").unwrap();

    assert!(matches!(
        import_word_list(&list, &dir.join("dicts"), &[&reference]),
        Err(DictionaryError::NoWordEntries)
    ));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn strips_known_extensions() {
    assert_eq!(strip_extensions("law.dict.yaml"), "law");
    assert_eq!(strip_extensions("dict.tsv"), "dict");
    assert_eq!(strip_extensions("x.qj"), "x");
    assert_eq!(strip_extensions("noext"), "noext");
}

/// 建一个临时目录，跑完删掉。
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qingjian-import-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn auto_import_derives_readings_for_word_lists() {
    let dir = scratch("auto-words");
    let reference = Dictionary::parse("李\tli\t900\n三\tsan\t900\n").unwrap();
    // 只有词的 txt：全部按参考词库派生
    let list = dir.join("names.txt");
    std::fs::write(&list, "李三\n囧三\n").unwrap();
    let done = import_dictionary(&list, &dir.join("dicts"), &[&reference]).unwrap();
    assert_eq!(done.imported.entries, 1);
    assert_eq!(done.report.unwrap().unknown, ["囧三".to_owned()]);

    // Rime 词库混了有读音的行：没读音的派生，写明的原样保留
    let rime = dir.join("mixed.dict.yaml");
    std::fs::write(&rime, "---\nname: mixed\n...\n李三\n三\tsan\t5\n").unwrap();
    let done = import_dictionary(&rime, &dir.join("dicts"), &[&reference]).unwrap();
    assert_eq!(done.imported.entries, 2);
    let dictionary = Dictionary::from_path(&done.imported.path).unwrap();
    assert_eq!(dictionary.lookup(&["li", "san"], false)[0].text, "李三");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn auto_import_keeps_normal_dictionaries_on_the_plain_path() {
    let dir = scratch("auto-plain");
    let reference = Dictionary::parse("李\tli\t900\n三\tsan\t900\n").unwrap();
    // 每行都有读音：仍走普通导入，返回的 report 为空
    let rime = dir.join("law.dict.yaml");
    std::fs::write(&rime, "---\nname: law\n...\n合同法\the tong fa\t120\n").unwrap();
    let done = import_dictionary(&rime, &dir.join("dicts"), &[&reference]).unwrap();
    assert!(done.report.is_none());
    assert_eq!(done.imported.name, "law");

    // 混进来的带数字声调的行也照原样保留（词表按无读音处理时同样认）
    let list = dir.join("toned.txt");
    std::fs::write(&list, "东西\tdong1 xi5\t20\n李三\n").unwrap();
    let done = import_dictionary(&list, &dir.join("dicts"), &[&reference]).unwrap();
    let dictionary = Dictionary::from_path(&done.imported.path).unwrap();
    assert_eq!(
        dictionary.lookup(&["dong", "xi"], false)[0].pinyin,
        "dong xi"
    );
    assert_eq!(dictionary.lookup(&["li", "san"], false)[0].text, "李三");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn imports_qjz_and_rejects_other_kinds() {
    let dir = std::env::temp_dir().join(format!("qingjian-import-qjz-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let tsv = dir.join("medicine.tsv");
    std::fs::write(&tsv, "一元论\tyi yuan lun\t20\n").unwrap();
    let packed = import(&tsv, &dir.join("build")).unwrap();
    let qjz_path = dir.join("domain-medicine.qjz");
    qjz::compress(&packed.path, &qjz_path).unwrap();

    let imported = import_dictionary(&qjz_path, &dir.join("dicts"), &[]).unwrap();
    assert_eq!(
        imported.imported.path.file_name().unwrap(),
        "domain-medicine.qj"
    );
    assert_eq!(imported.imported.entries, 1);
    assert!(imported.report.is_none());
    let dictionary = Dictionary::from_path(&imported.imported.path).unwrap();
    assert_eq!(
        dictionary.lookup(&["yi", "yuan", "lun"], false)[0].text,
        "一元论"
    );

    let glossary = dir.join("glossary.qj");
    qingjian_format::Writer::new(Kind::Glossary, &Metadata::default())
        .unwrap()
        .write_to(&glossary)
        .unwrap();
    let glossary_qjz = dir.join("glossary-ja.qjz");
    qjz::compress(&glossary, &glossary_qjz).unwrap();
    assert!(matches!(
        import(&glossary_qjz, &dir.join("dicts")),
        Err(DictionaryError::Format(
            qingjian_format::FormatError::WrongKind { .. }
        ))
    ));
    assert!(!dir.join("dicts/glossary-ja.qj").exists());
    std::fs::remove_dir_all(&dir).unwrap();
}
