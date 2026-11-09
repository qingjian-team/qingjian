//! 繁体输出：候选与上屏换成繁体，学习数据仍是简体。

use super::*;

use crate::traditional::TraditionalVariant;

/// 选一个中文候选并上屏，返回上屏的文本。
fn pick(engine: &mut Engine, input: &str) -> (Candidate, String) {
    engine.set_input(input);
    let candidate = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.kind == CandidateKind::Chinese)
        .unwrap();
    let committed = engine.commit(&candidate);
    (candidate, committed)
}

/// 候选窗口里是繁体、上屏也是繁体，但词频与「这段拼音选了什么」记的是简体原文：
/// 用户改回简体输出之后，之前学到的照样管用。
#[test]
fn candidates_and_commit_are_traditional_while_learning_stays_simplified() {
    let mut engine = engine().with_learner(Box::new(CountingLearner(HashMap::new())));
    engine.set_traditional(TraditionalVariant::Taiwan);
    let (candidate, committed) = pick(&mut engine, "kaifa");
    assert_eq!(candidate.text, "開發");
    assert_eq!(committed, "開發");
    assert_eq!(engine.learner().weight("开发"), 1);
    assert_eq!(engine.learner().weight("開發"), 0);
    assert_eq!(engine.learner().choice_weight("kaifa", "开发"), 1);
}

/// 关着的时候一个字都不变，学习也照旧。
#[test]
fn off_commits_simplified() {
    let mut engine = engine().with_learner(Box::new(CountingLearner(HashMap::new())));
    let (candidate, committed) = pick(&mut engine, "kaifa");
    assert_eq!(candidate.text, "开发");
    assert_eq!(committed, "开发");
    assert_eq!(engine.learner().weight("开发"), 1);
}

/// 候选按简体查释义表：转换之后译词还在。
#[test]
fn translations_still_resolve_after_conversion() {
    let mut engine = engine().with_translator(Box::new(FixedTranslator));
    engine.set_traditional(TraditionalVariant::Taiwan);
    engine.set_input("kaifa");
    let mut query = engine.query().unwrap();
    engine.annotate(&mut query.candidates);
    let candidate = query
        .candidates
        .items
        .iter()
        .find(|c| c.text == "開發")
        .unwrap();
    assert_eq!(
        candidate
            .translation
            .as_ref()
            .and_then(|t| t.senses().first())
            .map(|s| s.text.as_str()),
        Some("develop")
    );
}

/// 自定义短语是用户写的原文：开着繁体也一个字不改地上屏。
#[test]
fn custom_phrases_commit_verbatim() {
    let mut engine = engine();
    engine.set_traditional(TraditionalVariant::Taiwan);
    engine
        .set_custom_phrases(vec![crate::CustomPhrase::new("dz", "我的地址：软件园")])
        .unwrap();
    engine.set_input("dz");
    let candidate = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| matches!(c.kind, CandidateKind::Custom(_)))
        .unwrap();
    assert_eq!(candidate.text, "我的地址：软件园");
    assert_eq!(engine.commit(&candidate), "我的地址：软件园");
}
