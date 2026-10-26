//! 生词本页的筛选、标认识、等级与搜索。

use super::*;

use qingjian_core::Language;

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("qingjian-vocab-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn request(json: &str) -> VocabularyRequest {
    serde_json::from_str(json).unwrap()
}

#[test]
fn missing_book_is_empty() {
    let dir = temp_dir("missing");
    let page = vocabulary(&dir, &dir, &request(r#"{"language":"en"}"#));
    assert_eq!(page.seen, 0);
    assert!(page.words.is_empty());
}

#[test]
fn off_gives_an_empty_page() {
    let dir = temp_dir("off");
    let page = vocabulary(&dir, &dir, &request(r#"{"language":"off"}"#));
    assert!(page.words.is_empty());
}

#[test]
fn marking_known_filters_and_persists() {
    let dir = temp_dir("known");
    let today = jiff::Zoned::now().date();
    let mut book = VocabularyBook::open(dir.join("user-vocab.tsv"));
    for word in ["develop", "apple"] {
        book.record_exposure_on(Language::English, word, today);
    }
    book.save().unwrap();

    let page = vocabulary(
        &dir,
        &dir,
        &request(
            r#"{"language":"en","filter":"known","change":{"op":"known","word":"apple","known":true}}"#,
        ),
    );
    assert_eq!(page.matched, 1);
    assert_eq!(page.words[0].word, "apple");
    assert!(!page.words[0].fresh);

    let page = vocabulary(
        &dir,
        &dir,
        &request(r#"{"language":"en","filter":"fresh"}"#),
    );
    assert_eq!(
        page.words
            .iter()
            .map(|w| w.word.as_str())
            .collect::<Vec<_>>(),
        ["develop"]
    );

    let page = vocabulary(
        &dir,
        &dir,
        &request(r#"{"language":"en","change":{"op":"remove","word":"develop"}}"#),
    );
    assert_eq!(page.seen, 1);
}

#[test]
fn levels_come_from_the_level_table() {
    let dir = temp_dir("levels");
    std::fs::write(
        dir.join("levels-en.tsv"),
        "# levels\tA1\tA2\napple\tA1\ndevelop\tA2\n",
    )
    .unwrap();
    let today = jiff::Zoned::now().date();
    let mut book = VocabularyBook::open(dir.join("user-vocab.tsv"));
    for word in ["develop", "apple"] {
        book.record_exposure_on(Language::English, word, today);
    }
    book.save().unwrap();

    let page = vocabulary(&dir, &dir, &request(r#"{"language":"en","level":"A2"}"#));
    assert_eq!(page.levels, ["A1", "A2"]);
    assert_eq!(page.matched, 1);
    assert_eq!(page.words[0].level, "A2");
}

#[test]
fn search_matches_word_source_and_example_ignoring_case() {
    let dir = temp_dir("search");
    let today = jiff::Zoned::now().date();
    let mut book = VocabularyBook::open(dir.join("user-vocab.tsv"));
    book.record_example_on(
        Language::English,
        "develop",
        "开发",
        "我们开发了新功能",
        today,
    );
    book.record_example_on(Language::English, "apple", "苹果", "An Apple a day", today);
    book.save().unwrap();

    let words = |search: &str| {
        let json = format!(r#"{{"language":"en","search":"{search}"}}"#);
        vocabulary(&dir, &dir, &request(&json))
            .words
            .into_iter()
            .map(|w| w.word)
            .collect::<Vec<_>>()
    };
    assert_eq!(words("DEV"), ["develop"]);
    assert_eq!(words("苹果"), ["apple"]);
    assert_eq!(words("a day"), ["apple"]);
    assert_eq!(words(" "), words(""));
    assert!(words("zzz").is_empty());
}
