//! 形码用户词的保存、删除、容错与方案隔离。

use crate::FrequencyLearner;
use qingjian_core::Learner;

#[test]
fn learned_code_words_round_trip_and_forget() {
    let directory =
        std::env::temp_dir().join(format!("qingjian-code-words-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("frequency.tsv");
    let mut learner = FrequencyLearner::from_path(&path).unwrap();
    learner.learn_code_word("wubi", "甲乙", "abef");
    learner.learn_code_word("other", "甲乙", "zzzz");
    assert!(learner.user_words().is_none());
    assert!(learner.has_unsaved());
    learner.flush();
    assert!(!learner.has_unsaved());
    let mut learner = FrequencyLearner::from_path(&path).unwrap();
    assert_eq!(
        learner.user_code_words("wubi").unwrap().lookup("abef", 1)[0].text,
        "甲乙"
    );
    assert!(
        learner
            .user_code_words("other")
            .unwrap()
            .lookup("abef", 1)
            .is_empty()
    );
    assert!(FrequencyLearner::files(&path).contains(&directory.join("user-code-words.tsv")));
    assert!(learner.forget("甲乙").user_word);
    learner.flush();
    let learner = FrequencyLearner::from_path(&path).unwrap();
    assert!(learner.user_code_words("wubi").is_none());
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn malformed_rows_are_skipped_without_losing_valid_words() {
    let mut words = super::super::code_words::CodeWords::default();
    assert_eq!(words.load("wubi\t甲乙\tabef\nbroken\nwubi\t甲\ta\nwubi\t甲乙\t12\n../bad\t甲乙\tabef\nwubi\t乙丙\tefij\n"), 4);
    assert_eq!(words.table("wubi").unwrap().len(), 2);
    assert!(!words.dirty);
    words.learn("wubi", "甲乙", "abef");
    assert!(!words.dirty);
}
