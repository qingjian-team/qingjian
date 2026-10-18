use std::path::PathBuf;

use qingjian_core::{FRESH_UNTIL, Language, LevelCount, VocabularySummary, VocabularyTracker};
use qingjian_translate::LevelTable;

use super::*;

fn date(text: &str) -> Date {
    text.parse().unwrap()
}

/// 每个测试一个目录，测完删。
struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("qingjian-vocab-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn file(&self) -> PathBuf {
        self.0.join("user-vocab.tsv")
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn counts_exposures_and_summarizes_per_language() {
    let mut book = VocabularyBook::default();
    let en = Language::English;
    for _ in 0..FRESH_UNTIL {
        book.record_exposure_on(en, "develop", date("2026-08-01"));
    }
    book.record_exposure_on(en, "hello", date("2026-09-05"));
    book.record_commit_on(en, "hello", false, date("2026-09-05"));
    book.record_commit_on(en, "world", true, date("2026-09-06"));
    book.record_exposure_on(Language::Japanese, "開発する", date("2026-09-06"));
    assert_eq!(book.exposures(en, "develop"), FRESH_UNTIL);
    assert_eq!(book.exposures(en, "nothing"), 0);
    let summary = book.summary_on(en, date("2026-09-06"));
    assert_eq!(
        summary,
        VocabularySummary {
            seen: 3,
            familiar: 1,
            committed: 2,
            used: 1,
            new_this_week: 2,
            levels: Vec::new(),
        }
    );
    assert_eq!(
        book.summary_on(Language::Japanese, date("2026-09-06")).seen,
        1
    );
    assert_eq!(book.languages(), [Language::English, Language::Japanese]);
}

#[test]
fn splits_the_summary_by_level_when_a_table_is_attached() {
    let table = LevelTable::parse("# levels\tA1\tA2\nhello\tA1\nworld\tA1\ndevelop\tA2\n").unwrap();
    let mut book = VocabularyBook::default().with_levels(Language::English, table);
    let en = Language::English;
    for _ in 0..FRESH_UNTIL {
        book.record_exposure_on(en, "develop", date("2026-09-01"));
    }
    book.record_exposure_on(en, "hello", date("2026-09-06"));
    book.record_commit_on(en, "hello", false, date("2026-09-06"));
    book.record_exposure_on(en, "unlisted", date("2026-09-06"));
    let summary = book.summary_on(en, date("2026-09-06"));
    assert_eq!(summary.seen, 3);
    assert_eq!(
        summary.levels,
        vec![
            LevelCount {
                name: "A1".into(),
                total: 2,
                seen: 1,
                familiar: 0,
                committed: 1,
            },
            LevelCount {
                name: "A2".into(),
                total: 1,
                seen: 1,
                familiar: 1,
                committed: 0,
            },
        ]
    );
}

#[test]
fn round_trips_through_the_file_and_skips_bad_lines() {
    let dir = Dir::new("round-trip");
    let path = dir.file();
    std::fs::write(
        &path,
        "# 头\nen\tdevelop\t2\t1\t0\t2026-09-01\t2026-09-05\n坏行\nen\t\t1\t0\t0\t2026-09-01\t2026-09-01\n",
    )
    .unwrap();
    let mut book = VocabularyBook::open(&path);
    assert_eq!(book.len(), 1);
    assert_eq!(book.exposures(Language::English, "develop"), 2);
    book.save().unwrap();
    book.record_exposure_on(Language::English, "develop", date("2026-09-06"));
    book.record_commit_on(Language::English, "world", true, date("2026-09-06"));
    book.record_example_on(
        Language::English,
        "world",
        "世界",
        "你好\t世界。\n",
        date("2026-09-06"),
    );
    book.save().unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        text,
        "# 语言\t译词\t看到轮次\t上屏次数\t用过次数\t首见\t末见\t认识\t中文\t例句\n\
         en\tdevelop\t3\t1\t0\t2026-09-01\t2026-09-06\t0\t\t\n\
         en\tworld\t0\t1\t1\t2026-09-06\t2026-09-06\t0\t世界\t你好 世界。\n"
    );
    let reloaded = VocabularyBook::open(&path);
    let world = reloaded.get(Language::English, "world").unwrap();
    assert_eq!(
        (world.source.as_str(), world.example.as_str()),
        ("世界", "你好 世界。")
    );
    assert_eq!(
        reloaded
            .summary_on(Language::English, date("2026-09-06"))
            .seen,
        2
    );
}

#[test]
fn known_words_are_no_longer_fresh_and_count_as_familiar() {
    let mut book = VocabularyBook::default();
    let en = Language::English;
    book.record_exposure_on(en, "develop", date("2026-09-06"));
    assert!(book.set_known(en, "develop", true));
    assert!(!book.set_known(en, "missing", true));
    assert_eq!(book.exposures(en, "develop"), u32::MAX);
    assert_eq!(book.summary_on(en, date("2026-09-06")).familiar, 1);
}

/// 设置程序删词、标认识，输入法这边同时还在记：输入法写回前合并，两边的改动都在。
#[test]
fn refresh_merges_edits_made_by_the_other_side() {
    let dir = Dir::new("merge");
    let path = dir.file();
    let en = Language::English;
    let mut ime = VocabularyBook::open(&path);
    for word in ["develop", "world", "hello"] {
        ime.record_exposure_on(en, word, date("2026-09-06"));
    }
    ime.save().unwrap();

    // 设置程序那边
    let mut settings = VocabularyBook::open(&path);
    settings.refresh();
    settings.remove(en, "world");
    settings.set_known(en, "develop", true);
    settings.save().unwrap();

    // 输入法这边接着记，其中一条正是刚被删的
    ime.record_exposure_on(en, "develop", date("2026-09-07"));
    ime.record_exposure_on(en, "world", date("2026-09-07"));
    ime.record_exposure_on(en, "agent", date("2026-09-07"));
    ime.flush();

    let merged = VocabularyBook::open(&path);
    let words: Vec<_> = merged.words(en).map(|(word, _)| word).collect();
    assert_eq!(words, ["agent", "develop", "hello"]);
    let develop = merged.get(en, "develop").unwrap();
    assert!(develop.known);
    assert_eq!(develop.seen, 2);

    // 清空：输入法这边内存里的旧词也跟着没了
    let mut settings = VocabularyBook::open(&path);
    settings.clear(en);
    settings.save().unwrap();
    ime.flush();
    assert!(VocabularyBook::open(&path).is_empty());
}
