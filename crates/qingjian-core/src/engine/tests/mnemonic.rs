//! BIP-39 助记词防泄露：黑名单不落盘 + 连续命中转私密。见 `crate::engine::mnemonic`。

use super::{Candidate, Dictionary, Engine, Learner, SAMPLE, WordList, engine};

/// 跟 `english.rs` 里 `raw_committed_english_words_are_learned_and_come_back_as_candidates`
/// 用的是同一种桩：真记 `learn_english` 被叫过哪些词，比只看候选排序更直接。
#[derive(Default)]
struct EnglishLearner {
    words: Vec<String>,
    list: Option<WordList>,
}

impl Learner for EnglishLearner {
    fn record(&mut self, _candidate: &Candidate) {}
    fn weight(&self, _text: &str) -> u32 {
        0
    }
    fn learn_english(&mut self, word: &str) {
        self.words.push(word.to_owned());
        let tsv: String = self
            .words
            .iter()
            .map(|w| format!("{w}\t{w}\t1\n"))
            .collect();
        self.list = WordList::parse(&tsv).ok();
    }
    fn user_english(&self) -> Option<&WordList> {
        self.list.as_ref()
    }
}

/// 原样上屏一个字母串（回车），跟 `english.rs` 里的用法一样：先 query 一次再 take_raw。
fn commit_raw(engine: &mut Engine, word: &str) {
    engine.set_input(word);
    engine.query().unwrap();
    assert_eq!(engine.take_raw(), word);
}

#[test]
fn bip39_words_are_not_persisted_but_ordinary_english_words_are() {
    let mut engine = Engine::new(Dictionary::parse(SAMPLE).unwrap())
        .with_learner(Box::new(EnglishLearner::default()));

    // "abstract" 在 BIP-39 英文助记词表里：手打一遍不该进个人词库
    commit_raw(&mut engine, "abstract");
    assert!(
        engine
            .learner()
            .user_english()
            .and_then(|list| list.get("abstract"))
            .is_none(),
        "BIP-39 词表内的词不该被学习进个人词库"
    );

    // "gist" 不在表里：普通英文词学习行为不受影响
    commit_raw(&mut engine, "gist");
    assert_eq!(
        engine
            .learner()
            .user_english()
            .and_then(|list| list.get("gist")),
        Some("gist"),
        "表外的普通英文词应该照常学习"
    );
}

#[test]
fn bip39_matching_ignores_case() {
    let mut engine = Engine::new(Dictionary::parse(SAMPLE).unwrap())
        .with_learner(Box::new(EnglishLearner::default()));
    // 英文模式下敲的大写不该绕过黑名单
    engine.set_english_mode(true);
    commit_raw(&mut engine, "ABSTRACT");
    assert!(
        engine
            .learner()
            .user_english()
            .and_then(|list| list.get("abstract"))
            .is_none()
    );
}

/// 连续 6 个「学英文词」事件都命中 BIP-39 表，够 `BIP39_STREAK_THRESHOLD`，
/// 即使没有任何密码框声明，也该临时转成私密输入。
const STREAK_WORDS: [&str; 6] = [
    "abstract", "across", "actress", "address", "afraid", "angry",
];

#[test]
fn a_streak_of_bip39_words_triggers_private_input_without_any_password_field() {
    let mut engine = engine();
    assert!(!engine.is_private());

    for (index, word) in STREAK_WORDS.iter().enumerate() {
        commit_raw(&mut engine, word);
        let expect_private = index + 1 >= STREAK_WORDS.len();
        assert_eq!(
            engine.is_private(),
            expect_private,
            "敲到第 {} 个 BIP-39 词时私密状态应为 {expect_private}",
            index + 1
        );
    }

    // 接一个不在表里的英文词：连续命中打断，启发式私密退出
    commit_raw(&mut engine, "gist");
    assert!(!engine.is_private(), "命中中断后启发式私密应该自动退出");
}

#[test]
fn committing_chinese_between_bip39_words_breaks_the_streak() {
    let mut engine = engine();
    // 敲 5 个（差一个到阈值），中间插一次中文上屏，连续计数应该被打断
    for word in &STREAK_WORDS[..5] {
        commit_raw(&mut engine, word);
    }
    engine.set_input("kaifa");
    let candidate = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == "开发")
        .unwrap();
    engine.commit(&candidate);
    assert!(!engine.is_private(), "中文上屏打断了连续命中，不该转私密");

    // 打断之后重新数，5 个还不够
    for word in &STREAK_WORDS[..5] {
        commit_raw(&mut engine, word);
    }
    assert!(!engine.is_private());
}

#[test]
fn heuristic_private_never_overrides_a_password_field_declared_by_the_shell() {
    let mut engine = engine();
    // 壳先声明这是密码框
    engine.set_private(true);
    assert!(engine.is_private());

    // 期间敲的词打断了启发式那一路（其实从没启动过），但壳声明的私密不能被这一步误关掉
    commit_raw(&mut engine, "gist");
    assert!(
        engine.is_private(),
        "启发式那一路的『撤销』不该动壳声明的私密状态"
    );

    // 只有壳自己撤，才真正退出私密
    engine.set_private(false);
    assert!(!engine.is_private());
}
