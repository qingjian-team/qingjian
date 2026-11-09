//! 上屏后的联想：先个人接续再语言模型、只给短的汉字词、上屏联想词推进上文。

use super::*;

use crate::traditional::TraditionalVariant;

/// 「开发」后面常接「者」「了」，还有一个标点与一个过长的片段，联想里都不该出现。
struct FollowModel;

impl LanguageModel for FollowModel {
    fn log_prob(&self, _previous: Option<&str>, _word: &str) -> Option<f64> {
        None
    }

    fn successors(&self, previous: &str, limit: usize) -> Vec<String> {
        let words: &[&str] = match previous {
            "开发" => &["，", "了", "开发区管委会", "者"],
            "了" => &["吧"],
            _ => &[],
        };
        words.iter().take(limit).map(|w| (*w).to_owned()).collect()
    }
}

fn follow_engine() -> Engine {
    engine()
        .with_language_model(Box::new(FollowModel))
        .with_learner(Box::new(WordLearner::default()))
}

fn commit_first(engine: &mut Engine, input: &str) -> String {
    engine.set_input(input);
    let first = engine.query().unwrap().candidates.items[0].clone();
    engine.commit(&first)
}

#[test]
fn follow_ups_list_short_han_words_after_a_committed_word() {
    let mut engine = follow_engine();
    assert!(engine.follow_ups(8).is_empty(), "句首没有上文");
    assert_eq!(commit_first(&mut engine, "kaifa"), "开发");
    assert_eq!(engine.follow_ups(8), ["了", "者"]);
    assert_eq!(engine.follow_ups(1), ["了"]);
    // 还在组句时不给
    engine.set_input("kai");
    assert!(engine.follow_ups(8).is_empty());
}

#[test]
fn the_users_own_continuations_come_first() {
    let mut engine = follow_engine();
    commit_first(&mut engine, "kaifa");
    commit_first(&mut engine, "kafei");
    commit_first(&mut engine, "kaifa");
    assert_eq!(engine.follow_ups(8), ["咖啡", "了", "者"]);
}

#[test]
fn committing_a_follow_up_moves_the_context_on() {
    let mut engine = follow_engine();
    commit_first(&mut engine, "kaifa");
    assert_eq!(engine.commit_follow_up("了"), "了");
    assert_eq!(engine.follow_ups(8), ["吧"]);
    // 联想词照常记转移：下次「开发」后面「了」排第一
    commit_first(&mut engine, "kaifa");
    assert_eq!(engine.follow_ups(8)[0], "了");
}

#[test]
fn follow_ups_follow_traditional_output() {
    let mut engine = follow_engine();
    engine.set_traditional(TraditionalVariant::Taiwan);
    assert_eq!(commit_first(&mut engine, "kaifa"), "開發");
    assert_eq!(engine.follow_ups(8), ["了", "者"]);
    assert_eq!(engine.commit_follow_up("了"), "了");
    assert_eq!(engine.follow_ups(8), ["吧"]);
}
