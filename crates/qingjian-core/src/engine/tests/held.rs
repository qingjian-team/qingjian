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
    assert!(engine.backspace());
    assert_eq!(engine.held_text(), "开");
    assert_eq!(engine.composition().text(), "kaixian");
    assert!(engine.backspace());
    assert_eq!(engine.held_text(), "");
    assert_eq!(engine.composition().text(), "kaikaixian");
    assert_eq!(engine.composition().cursor(), 10);
    assert!(engine.backspace());
    assert_eq!(engine.composition().text(), "kaikaixia");
}

#[test]
fn undone_selection_retracts_its_learning_when_another_word_is_chosen() {
    let mut engine = engine().with_learner(Box::new(CountingLearner(HashMap::new())));
    engine.set_input("kaifangxian");
    pick(&mut engine, "开放");
    assert_eq!(engine.learner().weight("开放"), 1);
    engine.backspace();
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
    assert!(engine.backspace());
    assert_eq!(engine.composition().text(), "xia");
}

/// 按词跳：首选整句的第一个词留在组句里，候选换成剩下拼音的；退格撤回；单个词不跳。
#[test]
fn next_word_holds_the_first_word_of_the_sentence() {
    let dictionary = concat!(
        "交易\tjiao yi\t5000\n中心\tzhong xin\t5000\n忠心\tzhong xin\t3000\n",
        "开发\tkai fa\t5000\n",
    );
    let mut engine = Engine::new(Dictionary::parse(dictionary).unwrap());
    engine.set_input("jiaoyizhongxin");
    let lead = engine.query().unwrap().candidates.items.remove(0);
    assert_eq!(lead.text, "交易中心");
    assert_eq!(engine.hold_next_word(&lead).as_deref(), Some(""));
    assert_eq!(engine.held_text(), "交易");
    assert_eq!(engine.composition().text(), "zhongxin");
    let rest = engine.query().unwrap().candidates.items;
    assert!(rest.iter().any(|c| c.text == "忠心"));
    // 剩下的只有一个词，不再跳
    assert_eq!(engine.hold_next_word(&rest[0]), None);
    assert!(engine.backspace());
    assert_eq!(engine.held_text(), "");
    assert_eq!(engine.composition().text(), "jiaoyizhongxin");

    // 首选不是本地重算出的那句（知意重排、学过的整句）：按词库从句首切
    engine.set_input("jiaoyizhongxin");
    let mut reranked = engine.query().unwrap().candidates.items.remove(0);
    reranked.text = "交易忠心".to_owned();
    assert_eq!(engine.hold_next_word(&reranked).as_deref(), Some(""));
    assert_eq!(engine.held_text(), "交易");
    engine.clear();

    engine.set_input("kaifa");
    let word = engine.query().unwrap().candidates.items.remove(0);
    assert_eq!(engine.hold_next_word(&word), None);
    assert_eq!(engine.composition().text(), "kaifa");
}

/// 双拼下按词跳：定住的词吃掉它那几个键，剩下的键照常解码。
#[test]
fn next_word_works_with_shuangpin() {
    let mut engine = xiaohe();
    // 小鹤：kai = kd、fa = fa、xian = xm
    engine.set_input("kdfaxm");
    let lead = engine.query().unwrap().candidates.items.remove(0);
    assert_eq!(lead.kind, CandidateKind::Sentence, "{}", lead.text);
    assert_eq!(engine.hold_next_word(&lead).as_deref(), Some(""));
    assert_eq!(engine.held_text(), "开发");
    assert_eq!(engine.composition().text(), "xm");
}

/// 前面有定住的词时，光标在末尾就等于拼音行的字数：定住的字只算一次。
#[test]
fn cursor_stays_at_the_end_after_a_held_word() {
    let mut engine = engine();
    engine.set_input("kaifaxiankaifa");
    pick(&mut engine, "开发");
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "开发xian'kai'fa");
    assert_eq!(query.segments_cursor(), "开发xian'kai'fa".chars().count());

    // 双拼解码时也一样
    let mut engine = xiaohe();
    engine.set_input("kdfaxmkdfa");
    pick(&mut engine, "开发");
    let query = engine.query().unwrap();
    let text = query.marked_text();
    assert_eq!(query.segments_cursor(), text.chars().count(), "{text}");
}
