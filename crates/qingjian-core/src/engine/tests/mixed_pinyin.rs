//! 双拼兼容全拼的候选、消耗与编辑回归。

use super::*;

#[test]
fn mixed_full_pinyin_queries_and_commits_both_spellings() {
    let mut engine = Engine::new(
        Dictionary::parse(
            "你好\tni hao\t9000\n中国\tzhong guo\t8000\n你\tni\t1000\n好\thao\t1000\n",
        )
        .unwrap(),
    );
    engine.set_shuangpin(Some(Scheme::Xiaohe));
    engine.set_shuangpin_full_pinyin(true);
    for keys in [
        "nihc",
        "nihao",
        "ni'hao",
        "nihcvsgo",
        "nihaovsgo",
        "nihczhongguo",
    ] {
        engine.set_input(keys);
        let query = engine.query().unwrap();
        let candidate = query
            .candidates
            .items
            .iter()
            .find(|c| c.text == "你好")
            .unwrap()
            .clone();
        assert_eq!(
            query
                .candidates
                .items
                .iter()
                .filter(|c| c.text == "你好")
                .count(),
            1
        );
        assert_eq!(engine.commit(&candidate), "你好");
        let remaining = match keys {
            "nihcvsgo" | "nihaovsgo" => "vsgo",
            "nihczhongguo" => "zhongguo",
            _ => "",
        };
        assert_eq!(engine.composition().text(), remaining, "{keys}");
        if !remaining.is_empty() {
            let candidate = engine
                .query()
                .unwrap()
                .candidates
                .items
                .iter()
                .find(|c| c.text == "中国")
                .unwrap()
                .clone();
            engine.commit(&candidate);
            assert!(engine.composition().is_empty());
        }
    }
}

#[test]
fn mixed_full_pinyin_retains_ambiguous_candidates_and_learning() {
    let mut engine = xiaohe();
    engine.set_shuangpin_full_pinyin(true);
    engine.set_input("xian");
    let query = engine.query().unwrap();
    assert!(query.candidates.items.iter().any(|c| c.text == "西安"));
    let candidate = query
        .candidates
        .items
        .iter()
        .find(|c| c.text == "先")
        .unwrap()
        .clone();
    engine.commit(&candidate);
    assert!(engine.composition().is_empty());
    assert!(engine.recent_commits.last().unwrap().same_input("xian"));
    engine.set_shuangpin_full_pinyin(false);
    engine.set_input("xian");
    assert!(
        engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .all(|c| c.text != "先")
    );
}

#[test]
fn mixed_full_pinyin_preserves_raw_preedit_and_syllable_editing() {
    let mut engine = xiaohe();
    engine.set_shuangpin_full_pinyin(true);
    engine.set_shuangpin_raw_preedit(true);
    engine.set_input("kaifave");
    assert_eq!(engine.query().unwrap().marked_text(), "kaifave");
    engine.move_cursor_syllable_left();
    assert_eq!(engine.composition().cursor(), 5);
    engine.move_cursor_syllable_left();
    assert_eq!(engine.composition().cursor(), 3);
    assert_eq!(engine.query().unwrap().marked_cursor(), 3);
    engine.delete_syllable_backward();
    assert_eq!(engine.composition().text(), "fave");
    engine.move_cursor_syllable_right();
    assert_eq!(engine.composition().cursor(), 2);
    engine.set_input("bl");
    engine.move_cursor_syllable_left();
    assert_eq!(engine.composition().cursor(), 0);
    engine.move_cursor_syllable_right();
    assert_eq!(engine.composition().cursor(), 2);
}

#[test]
fn full_pinyin_candidates_survive_a_full_shuangpin_candidate_list() {
    let mut source = "先\txian\t10000\n".to_owned();
    for index in 0..MAX_CANDIDATES + 1 {
        source.push_str(&format!("西安{index}\txi an\t9000\n"));
    }
    let mut engine = Engine::new(Dictionary::parse(&source).unwrap());
    engine.set_shuangpin(Some(Scheme::Xiaohe));
    engine.set_shuangpin_full_pinyin(true);
    engine.set_input("xian");
    assert!(
        engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .take(9)
            .any(|c| c.text == "先")
    );
}
