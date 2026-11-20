//! 词表测试：查询、音标列、前缀补全、落盘对拍、错误路径与并发。

use super::*;

use qingjian_format::Metadata;

#[test]
fn looks_up_by_lowercase_code() {
    let list = WordList::parse("GitHub\tgithub\nhello\thello\niPhone\n").unwrap();
    assert_eq!(list.get("github"), Some("GitHub"));
    assert_eq!(list.get("iphone"), Some("iPhone"));
    assert_eq!(list.get("hello"), Some("hello"));
    assert_eq!(list.get("nope"), None);
}

/// 音标表与词表是两种格式：音标表第一列才是词。
#[test]
fn pronunciation_reads_the_second_column() {
    let list =
        WordList::parse_pronunciations("# 注释\ncargo\t/ˈkɑrɡoʊ/\nhello\t/həˈloʊ/\n").unwrap();
    assert_eq!(list.pronunciation("cargo"), Some("/ˈkɑrɡoʊ/"));
    assert_eq!(list.pronunciation("hello"), Some("/həˈloʊ/"));
    // 查不到音标的词返回 None，调用方据此不显示
    assert_eq!(list.pronunciation("abacuses"), None);
}

#[test]
fn completes_prefixes_by_frequency() {
    let list = WordList::parse(
        "compass\tcompass\t300\ncompany\tcompany\t900\ncompare\tcompare\t500\ncom\tcom\t100\ncomma\tcomma\n",
    )
    .unwrap();
    assert_eq!(list.complete("comp", 2), ["company", "compare"]);
    assert_eq!(
        list.complete("com", 10),
        ["company", "compare", "compass", "comma"]
    );
    assert!(list.complete("zzz", 3).is_empty());
    assert!(list.complete("", 3).is_empty());
}

#[test]
fn qj_round_trip_keeps_lookups() {
    let dir = std::env::temp_dir().join(format!("qingjian-word-list-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("words.qj");
    let source = "GitHub\tgithub\t700\ncompany\tcompany\t900\ncompare\tcompare\t500\ntiếng Việt\ttiếng việt\t20\n";
    WordList::parse(source)
        .unwrap()
        .write_qj(&path, &Metadata::default())
        .unwrap();
    let list = WordList::from_path(&path).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(list.len(), 4);
    assert_eq!(list.get("github"), Some("GitHub"));
    assert_eq!(list.frequency("company"), Some(900));
    assert_eq!(list.complete("comp", 5), ["company", "compare"]);
    assert_eq!(list.complete("tiếng", 5), ["tiếng Việt"]);
}

/// 随包的音标表是 `.qj`：打包后与解析 TSV 查到的一样。
#[test]
fn pronunciations_qj_round_trip() {
    let dir = std::env::temp_dir().join(format!("qingjian-ipa-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let tsv = dir.join("english-ipa.tsv");
    let qj = dir.join("english-ipa.qj");
    std::fs::write(&tsv, "Cargo\t/ˈkɑrɡoʊ/\naccede\t/ækˈsid/\n").unwrap();
    WordList::pronunciations_from_path(&tsv)
        .unwrap()
        .write_qj(&qj, &Metadata::default())
        .unwrap();
    let list = WordList::pronunciations_from_path(&qj).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(list.pronunciation("cargo"), Some("/ˈkɑrɡoʊ/"));
    assert_eq!(list.pronunciation("accede"), Some("/ækˈsid/"));
}

fn lcg(mut state: u64) -> impl FnMut(usize) -> usize {
    move |bound| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((state >> 33) as usize) % bound
    }
}

/// 随机条目（含非 ASCII、空格、原样大小写）落盘再打开：条目、查询与前缀补全一致。
#[test]
fn qj_round_trip_preserves_random_entries() {
    let mut next = lcg(0x3C6E_F372_9C14_5B6D);
    let words = [
        "GitHub",
        "iPhone",
        "hello",
        "tiếng Việt",
        "café",
        "a b",
        "apple",
        "compose",
    ];
    let mut tsv = String::new();
    for word in words {
        let frequency = 1 + next(500);
        // 一半给显式编码，一半让编码缺省成小写词形。
        if next(2) == 0 {
            tsv.push_str(&format!(
                "{word}\t{}\t{frequency}\n",
                word.to_ascii_lowercase()
            ));
        } else {
            tsv.push_str(&format!("{word}\t{frequency}\n"));
        }
    }
    let original = WordList::parse(&tsv).unwrap();

    let dir =
        std::env::temp_dir().join(format!("qingjian-word-list-random-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("random.qj");
    original.write_qj(&path, &Metadata::default()).unwrap();
    let reopened = WordList::from_path(&path).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();

    assert_eq!(reopened.len(), original.len());
    let snapshot = |list: &WordList| {
        let mut rows: Vec<(String, String, u32)> = list
            .entries()
            .map(|(code, word, frequency)| (code.to_owned(), word.to_owned(), frequency))
            .collect();
        rows.sort();
        rows
    };
    assert_eq!(snapshot(&reopened), snapshot(&original));
    for code in ["github", "iphone", "tiếng việt", "comp"] {
        assert_eq!(reopened.get(code), original.get(code));
        assert_eq!(reopened.complete(code, 10), original.complete(code, 10));
    }
}

#[test]
fn empty_list_and_missing_file_are_handled() {
    let list = WordList::parse("").unwrap();
    assert!(list.is_empty());
    assert!(list.get("anything").is_none());
    assert!(list.complete("a", 5).is_empty());

    let dir = std::env::temp_dir().join(format!("qingjian-word-list-err-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let missing = dir.join("missing.qj");
    assert!(WordList::from_path(&missing).is_err());
    std::fs::remove_dir_all(&dir).unwrap();
}

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn word_list_is_send_sync_and_concurrent_reads_are_stable() {
    assert_send_sync::<WordList>();
    let list = std::sync::Arc::new(
        WordList::parse("compass\tcompass\t300\ncompany\tcompany\t900\ncompare\tcompare\t500\n")
            .unwrap(),
    );
    let owned = |list: &WordList| -> (Option<String>, Vec<String>) {
        (
            list.get("company").map(str::to_owned),
            list.complete("com", 10)
                .iter()
                .map(|s| s.to_string())
                .collect(),
        )
    };
    let baseline = owned(&list);
    let mut handles = Vec::new();
    for _ in 0..8 {
        let list = std::sync::Arc::clone(&list);
        handles.push(std::thread::spawn(move || {
            (0..50).map(|_| owned(&list)).collect::<Vec<_>>()
        }));
    }
    for handle in handles {
        for snapshot in handle.join().unwrap() {
            assert_eq!(snapshot, baseline);
        }
    }
}
