//! 分几次选完一段拼音：前面选的词留在组句里，退格先撤回选择。

use super::*;

fn pick(engine: &mut Engine, text: &str) -> String {
    let candidate = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == text && c.kind == CandidateKind::Chinese)
        .unwrap();
    engine.commit(&candidate)
}

#[test]
fn partial_selection_is_held_until_the_buffer_is_used_up() {
    let mut engine = engine();
    engine.set_input("kaifaxian");
    assert_eq!(pick(&mut engine, "开发"), "");
    assert_eq!(engine.held_text(), "开发");
    assert_eq!(engine.composition().text(), "xian");
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "开发xian");
    assert_eq!(query.segments_cursor(), 6);
    assert_eq!(pick(&mut engine, "先"), "开发先");
    assert_eq!(engine.held_text(), "");
}

#[test]
fn backspace_undoes_selections_before_deleting_letters() {
    let mut engine = engine();
    engine.set_input("kaikaixian");
    pick(&mut engine, "开");
    pick(&mut engine, "开");
    assert_eq!(engine.held_text(), "开开");
    assert!(engine.backspace().consumed());
    assert_eq!(engine.held_text(), "开");
    assert_eq!(engine.composition().text(), "kaixian");
    assert!(engine.backspace().consumed());
    assert_eq!(engine.held_text(), "");
    assert_eq!(engine.composition().text(), "kaikaixian");
    assert_eq!(engine.composition().cursor(), 10);
    assert!(engine.backspace().consumed());
    assert_eq!(engine.composition().text(), "kaikaixia");
}

#[test]
fn undone_selection_retracts_its_learning_when_another_word_is_chosen() {
    let mut engine = engine().with_learner(Box::new(CountingLearner(HashMap::new())));
    engine.set_input("kaifangxian");
    pick(&mut engine, "开放");
    assert_eq!(engine.learner().weight("开放"), 1);
    engine.backspace().consumed();
    pick(&mut engine, "开");
    assert_eq!(engine.learner().weight("开放"), 0);
    assert_eq!(engine.learner().weight("开"), 1);
}

#[test]
fn raw_commit_and_clear_include_or_drop_held_words() {
    let mut engine = engine();
    engine.set_input("kaifaxian");
    pick(&mut engine, "开发");
    assert_eq!(engine.take_raw(), "开发xian");
    assert_eq!(engine.held_text(), "");

    engine.set_input("kaifaxian");
    pick(&mut engine, "开发");
    engine.clear();
    assert_eq!(engine.held_text(), "");
    assert!(engine.composition().is_empty());

    // 组句中敲标点：壳先取走留着的词再补标点，剩下的拼音不动
    engine.set_input("kaifaxian");
    pick(&mut engine, "开发");
    assert_eq!(engine.take_held(), "开发");
    assert_eq!(engine.composition().text(), "xian");
}

#[test]
fn holding_can_be_turned_off() {
    let mut engine = engine();
    engine.set_hold_selections(false);
    engine.set_input("kaifaxian");
    assert_eq!(pick(&mut engine, "开发"), "开发");
    assert!(engine.backspace().consumed());
    assert_eq!(engine.composition().text(), "xia");
}

/// 长按保护：组句被删空之后那一记重复要被吞掉，别让它删到应用里的字。
///
/// 真实时钟下连按的间隔是零，正好落在窗口里；窗口本身设短一点，测试不必真的等 250 ms。
#[test]
fn backspace_holds_after_emptying_the_buffer() {
    use crate::engine::Backspace;
    use std::time::Duration;

    let mut engine = engine().with_backspace_hold(Duration::from_millis(250));
    engine.set_input("ni");
    assert_eq!(engine.backspace(), Backspace::Deleted);
    assert_eq!(engine.backspace(), Backspace::Deleted);
    // 这一下把组句删空了，后面的重复该停在输入法里
    assert_eq!(engine.backspace(), Backspace::Held);
    assert_eq!(engine.backspace(), Backspace::Held);
    assert!(engine.composition().is_empty());
}

/// 窗口是零就是不保护：一次退格一次删除，壳立刻把空缓冲区之后的键交给应用。
#[test]
fn backspace_without_hold_releases_right_away() {
    use crate::engine::Backspace;

    let mut engine = engine();
    engine.set_input("ni");
    assert_eq!(engine.backspace(), Backspace::Deleted);
    assert_eq!(engine.backspace(), Backspace::Deleted);
    assert_eq!(engine.backspace(), Backspace::Released);
}
