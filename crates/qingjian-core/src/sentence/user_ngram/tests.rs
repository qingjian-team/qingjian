use super::*;

#[test]
fn counts_transitions_and_round_trips_through_tsv() {
    let mut model = UserNgram::default();
    model.record(Context::START, "我");
    model.record(Context::after("我"), "想");
    model.record(Context::after_two("我", "想"), "去");
    model.record(Context::START, "我");
    model.record(Context::after("我"), "想");
    assert_eq!(model.count("我"), 2);
    assert_eq!(model.count("想"), 2);
    assert_eq!(model.pair(None, "我"), 2);
    assert_eq!(model.pair(Some("我"), "想"), 2);
    assert_eq!(model.pair(Some("我"), "去"), 0);
    assert_eq!(model.pair_count(), 3);
    // 句首词不记三元；第二个词的前二词是句首标记
    assert_eq!(model.triple(None, "我", "想"), 2);
    assert_eq!(model.triple(Some("我"), "想", "去"), 1);
    assert_eq!(model.triple_count(), 2);

    let tsv = model.to_tsv();
    assert!(tsv.contains("<s>\t我\t2\n"));
    assert!(tsv.contains("<s>\t我\t想\t2\n"));
    assert!(tsv.contains("我\t想\t去\t1\n"));
    let restored = UserNgram::parse(&tsv).unwrap();
    assert_eq!(restored.pair(Some("我"), "想"), 2);
    assert_eq!(restored.count("去"), 1);
    assert_eq!(restored.pair_count(), 3);
    assert_eq!(restored.triple(None, "我", "想"), 2);
    assert_eq!(restored.triple_count(), 2);
    assert_eq!(restored.to_tsv(), tsv);
    assert_eq!(UserNgram::parse("我\t想\n").unwrap_err(), 1);
    assert_eq!(UserNgram::parse("<s>\t我\t想\t去\t1\n").unwrap_err(), 1);
    // 宽松解析：坏行跳过，好行照读
    let (model, skipped) = UserNgram::parse_lenient("<s>\t我\t3\n坏行\n我\t想\tx\n我\t想\t2\n");
    assert_eq!(skipped, [2, 3]);
    assert_eq!(model.pair(None, "我"), 3);
    assert_eq!(model.pair(Some("我"), "想"), 2);
}

/// 旧版只有二元的文件照读，三元表为空。
#[test]
fn parses_bigram_only_files() {
    let model = UserNgram::parse("<s>\t我\t3\n我\t想\t2\n").unwrap();
    assert_eq!(model.pair(Some("我"), "想"), 2);
    assert_eq!(model.triple_count(), 0);
    assert_eq!(
        model.blend(
            Context::after_two("我", "想"),
            "去",
            -5.0,
            &Interpolation::DEFAULT
        ),
        model.blend(Context::after("想"), "去", -5.0, &Interpolation::DEFAULT)
    );
}

#[test]
fn blend_favors_seen_continuations_but_never_kills_unseen_ones() {
    let mut model = UserNgram::default();
    let base_ba = (-9.0_f64).exp().ln();
    // 前词没见过：原样返回
    assert_eq!(
        model.blend(
            Context::after("吃饭"),
            "把",
            base_ba,
            &Interpolation::DEFAULT
        ),
        base_ba
    );
    // 选过两次 吃饭 → 把：个人证据抬上来
    model.record(Context::after("吃饭"), "把");
    model.record(Context::after("吃饭"), "把");
    let lifted = model.blend(
        Context::after("吃饭"),
        "把",
        base_ba,
        &Interpolation::DEFAULT,
    );
    assert!(lifted > -2.5, "{lifted}");
    // 没跟在 吃饭 后面出现过的 吧 只是打折，不会被压死
    let base_ba_particle = -2.0;
    let discounted = model.blend(
        Context::after("吃饭"),
        "吧",
        base_ba_particle,
        &Interpolation::DEFAULT,
    );
    assert!(discounted < base_ba_particle);
    assert!(
        discounted > base_ba_particle + (1.0 - Interpolation::DEFAULT.max_confidence).ln() - 1e-9
    );
}

/// 三元分辨二元分不开的接续：「想 → 去」在「我想」后面和「不想」后面偏好不同。
#[test]
fn trigram_separates_continuations_the_bigram_mixes() {
    let mut model = UserNgram::default();
    for _ in 0..3 {
        model.record(Context::after_two("我", "想"), "去");
        model.record(Context::after_two("不", "想"), "要");
    }
    let base = -6.0;
    let after_wo = model.blend(
        Context::after_two("我", "想"),
        "去",
        base,
        &Interpolation::DEFAULT,
    );
    let after_bu = model.blend(
        Context::after_two("不", "想"),
        "去",
        base,
        &Interpolation::DEFAULT,
    );
    let bigram_only = model.blend(Context::after("想"), "去", base, &Interpolation::DEFAULT);
    assert!(after_wo > bigram_only, "{after_wo} vs {bigram_only}");
    assert!(bigram_only > after_bu, "{bigram_only} vs {after_bu}");
    // 三元上文没见过的（「很想」）退回二元
    assert_eq!(
        model.blend(
            Context::after_two("很", "想"),
            "去",
            base,
            &Interpolation::DEFAULT
        ),
        bigram_only
    );
    // 没见过的接续在见过的三元上文里仍拿到回退份额，不会被压死
    let unseen = model.blend(
        Context::after_two("我", "想"),
        "要",
        base,
        &Interpolation::DEFAULT,
    );
    assert!(unseen > base + (1.0 - Interpolation::DEFAULT.max_confidence).ln() - 1e-9);
    assert!(unseen < after_wo);
}

#[test]
fn forget_word_removes_it_from_every_position() {
    let mut model = UserNgram::default();
    model.record(Context::START, "我");
    model.record(Context::after("我"), "想");
    model.record(Context::after_two("我", "想"), "去");
    model.record(Context::after("我"), "去");
    assert_eq!(model.forget_word("想"), 2);
    assert_eq!(model.count("想"), 0);
    assert_eq!(model.pair(Some("我"), "想"), 0);
    assert_eq!(model.pair(Some("想"), "去"), 0);
    assert_eq!(model.pair(Some("我"), "去"), 1);
    assert_eq!(model.count("去"), 1);
    assert_eq!(model.pair_count(), 2);
    assert_eq!(model.triple(None, "我", "想"), 0);
    assert_eq!(model.triple(Some("我"), "想", "去"), 0);
    assert_eq!(model.triple(None, "我", "去"), 1);
    assert_eq!(model.triple_count(), 1);
    assert_eq!(model.forget_word("没有"), 0);
    // 作为前二词删掉
    assert_eq!(model.forget_word("我"), 2);
    assert_eq!(model.triple_count(), 0);
    assert!(model.is_empty());
}

#[test]
fn decays_when_too_many_transitions() {
    let mut model = UserNgram::default();
    // 每条记录一个二元加一个三元
    for i in 0..MAX_USER_TRANSITIONS / 2 {
        model.record(Context::after("甲"), &format!("乙{i}"));
    }
    for _ in 0..3 {
        model.record(Context::after("甲"), "丙");
    }
    // 超上限后减半：单次的都被忘掉，选过三次的还留着
    model.record(Context::after("甲"), "丁");
    assert!(model.transition_count() <= 4);
    assert!(model.pair(Some("甲"), "丙") >= 1);
    assert!(model.triple(None, "甲", "丙") >= 1);
    assert_eq!(model.pair(Some("甲"), "乙0"), 0);
    assert_eq!(model.triple(None, "甲", "乙0"), 0);
}

#[test]
fn unrecord_reverses_record_and_ignores_unknown_transitions() {
    let mut model = UserNgram::default();
    model.record_times(Context::START, "好的", 2);
    model.record(Context::after("好的"), "啊");
    model.unrecord(Context::START, "好的", 2);
    assert_eq!(model.pair(None, "好的"), 0);
    assert_eq!(model.count("好的"), 0);
    assert_eq!(model.pair(Some("好的"), "啊"), 1);
    assert_eq!(model.triple(None, "好的", "啊"), 1);
    assert_eq!(model.pair_count(), 1);
    model.unrecord(Context::after("没有"), "的", 5);
    model.unrecord(Context::after("好的"), "啊", 5);
    assert!(model.is_empty());
    assert_eq!(model.transition_count(), 0);
}
