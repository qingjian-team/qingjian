//! 导出清单与清空学习数据各删哪些文件。

use super::*;

struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "qingjian-android-learning-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("dicts")).unwrap();
        for file in [
            "user.tsv",
            "user-days.tsv",
            "user-ngram.tsv",
            "user-vocab.tsv",
            "vocab-log.tsv",
            "usage-hourly.tsv",
            "phrases.json",
            "clipboard.jsonl",
            "api-usage.lock",
            "dicts/law.qj",
        ] {
            std::fs::write(dir.join(file), "x").unwrap();
        }
        Self(dir)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn export_skips_clipboard_locks_and_dictionaries() {
    let dir = Dir::new("export");
    let names: Vec<String> = export_files(&dir.0)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(
        names,
        [
            "phrases.json",
            "usage-hourly.tsv",
            "user-days.tsv",
            "user-ngram.tsv",
            "user-vocab.tsv",
            "user.tsv",
            "vocab-log.tsv",
        ]
    );
}

#[test]
fn clear_habits_keeps_vocabulary_and_stats() {
    let dir = Dir::new("habits");
    assert_eq!(clear(&dir.0, true, false), 3);
    assert!(!dir.0.join("user.tsv").exists());
    assert!(!dir.0.join("user-ngram.tsv").exists());
    assert!(dir.0.join("user-vocab.tsv").exists());
    assert!(dir.0.join("usage-hourly.tsv").exists());
    assert!(dir.0.join("phrases.json").exists());
}

#[test]
fn clear_vocabulary_removes_book_and_timeline() {
    let dir = Dir::new("vocabulary");
    assert_eq!(clear(&dir.0, false, true), 2);
    assert!(!dir.0.join("user-vocab.tsv").exists());
    assert!(!dir.0.join("vocab-log.tsv").exists());
    assert!(dir.0.join("user.tsv").exists());
}
