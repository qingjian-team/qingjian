use super::*;

#[test]
fn zhuyin_decodes_keys_before_lookup_and_shows_zhuyin_symbols() {
    let mut engine = engine();
    engine.set_zhuyin_mode(true);

    // 1: ㄅ, j: ㄨ, 4: ˋ -> 不 (bu)
    engine.set_input("1j4");

    let query = engine.query().unwrap();
    // 顯示上屏前的標記文字應該是注音符號
    assert_eq!(query.marked_text(), "ㄅㄨˋ");
    assert_eq!(query.marked_cursor(), 3);
    assert!(query.decoded_keys);

    // 驗證候選詞裡面包含「不」（拼音為 bu）
    let has_bu = query.candidates.items.iter().any(|c| c.text == "不");
    assert!(has_bu, "Candidate '不' should be present");

    // 驗證 take_raw() 回傳注音符號（對應修改的第 1 條規則）
    assert_eq!(engine.take_raw(), "ㄅㄨˋ");
}

/// 词频故意按 吗 > 马 > 妈 排：注音敲了声调时要按声调排，全拼照旧按词频
fn tone_engine() -> Engine {
    Engine::new(
        Dictionary::parse("吗\tma\t9000\n马\tma\t5000\n妈\tma\t1000\n你\tni\t9000\n呢\tni\t8000\n")
            .unwrap(),
    )
}

fn first_candidate(engine: &mut Engine, keys: &str) -> String {
    engine.set_input(keys);
    engine.query().unwrap().candidates.items[0].text.clone()
}

#[test]
fn zhuyin_ranks_candidates_by_typed_tone() {
    let mut engine = tone_engine();
    engine.set_zhuyin_mode(true);
    // a: ㄇ, 8: ㄚ, 空格：一声，3: ˇ，7: ˙；s: ㄋ, u: ㄧ
    assert_eq!(first_candidate(&mut engine, "a8 "), "妈");
    assert_eq!(first_candidate(&mut engine, "a83"), "马");
    assert_eq!(first_candidate(&mut engine, "a87"), "吗");
    assert_eq!(first_candidate(&mut engine, "su3"), "你");
    // 没敲声调不限
    assert_eq!(first_candidate(&mut engine, "a8"), "吗");
}

#[test]
fn pinyin_ranking_ignores_tone_table() {
    let mut engine = tone_engine();
    assert_eq!(first_candidate(&mut engine, "ma"), "吗");
}

#[test]
fn zhuyin_sentence_follows_typed_tone() {
    let dictionary =
        "我\two\t9000\n想\txiang\t8000\n买\tmai\t9000\n卖\tmai\t3000\n我想\two xiang\t5000\n";
    let mut engine = Engine::new(Dictionary::parse(dictionary).unwrap());
    engine.set_zhuyin_mode(true);
    // ㄨㄛˇ ㄒㄧㄤˇ ㄇㄞˇ / ㄇㄞˋ：只差最后一个声调
    assert_eq!(first_candidate(&mut engine, "ji3vu;3a93"), "我想买");
    assert_eq!(first_candidate(&mut engine, "ji3vu;3a94"), "我想卖");
    // 全拼没有声调，照旧按词频
    engine.set_zhuyin_mode(false);
    assert_eq!(first_candidate(&mut engine, "woxiangmai"), "我想买");
}
