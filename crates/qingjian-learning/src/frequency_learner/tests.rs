use super::*;

fn candidate(text: &str) -> Candidate {
    Candidate {
        text: text.to_owned(),
        kind: qingjian_core::CandidateKind::Chinese,
        syllables: Vec::new(),
        reading: None,
        translation: None,
        aux_code: None,
    }
}

#[test]
fn parse_keeps_counts_as_fresh() {
    let learner = FrequencyLearner::parse("我\t3\n");
    assert_eq!(learner.weight("我"), 3);
}

#[test]
fn records_and_round_trips_through_tsv() {
    let mut learner = FrequencyLearner::default();
    learner.record(&candidate("开发"));
    learner.record(&candidate("开发"));
    learner.record(&candidate("中文"));
    assert_eq!(learner.weight("开发"), 2);
    assert!(learner.is_dirty());

    let dir = std::env::temp_dir().join(format!("qingjian-learning-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user.tsv");
    learner.save_to(&path).unwrap();
    assert!(!learner.is_dirty());

    let mut restored = FrequencyLearner::from_path(&path).unwrap();
    assert_eq!(restored.weight("开发"), 2);
    assert_eq!(restored.weight("中文"), 1);
    assert_eq!(restored.weight("没有"), 0);

    // flush 写回加载时的路径
    restored.record(&candidate("中文"));
    restored.flush();
    assert_eq!(
        FrequencyLearner::from_path(&path).unwrap().weight("中文"),
        2
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn choices_are_keyed_by_input_and_round_trip() {
    let dir = std::env::temp_dir().join("qingjian-user-choices-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user.tsv");
    let _ = std::fs::remove_file(dir.join(USER_CHOICES_FILE));

    let mut learner = FrequencyLearner::from_path(&path).unwrap();
    learner.record_choice("ba", "", "吧");
    learner.record_choice("ba", "", "吧");
    learner.record_choice("bazhege", "", "把");
    learner.record_raw("nihooma");
    assert_eq!(learner.raw_count("nihooma"), 1);
    assert_eq!(learner.choice_weight("ba", "吧"), 2);
    assert_eq!(learner.choice_weight("ba", "把"), 0);
    assert_eq!(learner.choice_weight("bazhege", "把"), 1);
    learner.flush();

    let reloaded = FrequencyLearner::from_path(&path).unwrap();
    assert_eq!(reloaded.choice_count(), 3);
    assert_eq!(reloaded.choice_weight("ba", "吧"), 2);
    assert_eq!(reloaded.raw_count("nihooma"), 1);
    assert_eq!(reloaded.choice_weight("bazhege", "把"), 1);
}

#[test]
fn demotions_add_up_undo_and_round_trip() {
    let dir = std::env::temp_dir().join("qingjian-user-demoted-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user.tsv");
    let _ = std::fs::remove_file(dir.join(USER_DEMOTED_FILE));

    let mut learner = FrequencyLearner::from_path(&path).unwrap();
    learner.demote("qj", "前几");
    learner.demote("qj", "前几");
    learner.demote("qj", "千金");
    learner.demote("qianji", "前几");
    assert_eq!(learner.demotion("qj", "前几"), 2);
    assert_eq!(learner.demotion("qianji", "千金"), 0);
    assert!(learner.undemote("qj", "千金"));
    assert!(!learner.undemote("qj", "千金"));
    learner.flush();

    let reloaded = FrequencyLearner::from_path(&path).unwrap();
    assert_eq!(reloaded.demoted_count(), 2);
    assert_eq!(reloaded.demotion("qj", "前几"), 2);
    assert_eq!(reloaded.demotion("qianji", "前几"), 1);
    assert_eq!(reloaded.demotion("qj", "千金"), 0);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn unrecord_reverses_each_kind_of_record() {
    let mut learner = FrequencyLearner::default();
    let candidate = Candidate {
        text: "开放".into(),
        kind: qingjian_core::CandidateKind::Chinese,
        syllables: vec!["kai".into(), "fang".into()],
        reading: None,
        translation: None,
        aux_code: None,
    };
    learner.record(&candidate);
    learner.record_choice("kaifa", "", "开放");
    learner.record_transition(Context::START, "开放", 2);
    learner.unrecord("开放");
    learner.unrecord_choice("kaifa", "", "开放");
    learner.unrecord_transition(Context::START, "开放", 2);
    assert_eq!(learner.weight("开放"), 0);
    assert_eq!(learner.choice_weight("kaifa", "开放"), 0);
    assert_eq!(learner.choice_count(), 0);
    assert!(learner.user_ngram().is_none());
    // 没记过的撤销不会变成负数
    learner.unrecord("没有");
    learner.unrecord_choice("x", "", "没有");
    assert_eq!(learner.weight("没有"), 0);
}

#[test]
fn choices_decay_when_over_the_cap() {
    let mut learner = FrequencyLearner::default();
    learner.record_choice("a", "", "甲");
    learner.record_choice("a", "", "甲");
    learner.record_choice("a", "", "甲");
    learner.record_choice("a", "", "乙");
    learner.decay_choices();
    assert_eq!(learner.choice_weight("a", "甲"), 1);
    assert_eq!(learner.choice_weight("a", "乙"), 0);
    assert_eq!(learner.choice_count(), 1);
}

#[test]
fn transitions_round_trip_through_ngram_tsv() {
    let dir = std::env::temp_dir().join("qingjian-user-ngram-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user.tsv");
    let _ = std::fs::remove_file(dir.join(USER_NGRAM_FILE));

    let mut learner = FrequencyLearner::from_path(&path).unwrap();
    assert!(learner.user_ngram().is_none());
    learner.record_transition(Context::START, "我", 1);
    learner.record_transition(Context::after("我"), "想", 1);
    learner.record_transition(Context::after("我"), "想", 1);
    learner.record_transition(Context::after_two("我", "想"), "去", 1);
    assert_eq!(learner.user_ngram().unwrap().pair(Some("我"), "想"), 2);
    learner.flush();

    let reloaded = FrequencyLearner::from_path(&path).unwrap();
    // 二元 <s>我 / 我想 / 想去，三元 <s>我想 / 我想去
    assert_eq!(reloaded.ngram_transition_count(), 5);
    assert_eq!(reloaded.user_ngram().unwrap().pair(None, "我"), 1);
    assert_eq!(reloaded.user_ngram().unwrap().count("想"), 2);
    assert_eq!(
        reloaded
            .user_ngram()
            .unwrap()
            .triple(Some("我"), "想", "去"),
        1
    );
}

#[test]
fn typos_round_trip_and_unrecord() {
    let dir = std::env::temp_dir().join("qingjian-user-typos-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user.tsv");
    let _ = std::fs::remove_file(dir.join(USER_TYPOS_FILE));

    let mut learner = FrequencyLearner::from_path(&path).unwrap();
    learner.record_typo("gan", "guan");
    learner.record_typo("gan", "guan");
    learner.record_typo("shou", "shuo");
    learner.unrecord_typo("shou", "shuo");
    learner.unrecord_typo("mei", "you");
    assert_eq!(learner.typo_count("gan", "guan"), 2);
    assert_eq!(learner.typo_count("shou", "shuo"), 0);
    assert_eq!(learner.typo_count_total(), 1);
    learner.flush();

    let reloaded = FrequencyLearner::from_path(&path).unwrap();
    assert_eq!(reloaded.typo_count("gan", "guan"), 2);
    assert_eq!(reloaded.typo_count_total(), 1);
}

#[test]
fn broken_lines_are_skipped_instead_of_failing_the_load() {
    let dir = std::env::temp_dir().join("qingjian-broken-lines-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user.tsv");
    std::fs::write(&path, "开发\t3\n没有次数\n中文\tabc\n中文\t1\n").unwrap();
    std::fs::write(dir.join(USER_NGRAM_FILE), "<s>\t我\t3\n坏行\n我\t想\t2\n").unwrap();
    std::fs::write(dir.join(USER_CHOICES_FILE), "ba\t吧\t2\nba\n").unwrap();
    std::fs::write(dir.join(USER_TYPOS_FILE), "gan\tguan\tx\ngan\tguan\t1\n").unwrap();
    std::fs::write(dir.join(USER_WORDS_FILE), "账套\tzhang tao\t100\n只有词\n").unwrap();
    // 编码坏掉的字节也不能让整个文件读不了
    std::fs::write(dir.join(USER_ENGLISH_FILE), b"gist\t2\n\xff\xfe\t1\n").unwrap();

    let learner = FrequencyLearner::from_path(&path).unwrap();
    assert_eq!(learner.weight("开发"), 3);
    assert_eq!(learner.weight("中文"), 1);
    assert_eq!(learner.user_ngram().unwrap().pair(Some("我"), "想"), 2);
    assert_eq!(learner.choice_weight("ba", "吧"), 2);
    assert_eq!(learner.typo_count("gan", "guan"), 1);
    assert_eq!(learner.word_count(), 1);
    assert_eq!(learner.english_count(), 2);
    assert!(learner.path.is_some());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn saving_is_atomic_and_leaves_no_temporary_files() {
    let dir = std::env::temp_dir().join("qingjian-atomic-save-test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user.tsv");
    let mut learner = FrequencyLearner::from_path(&path).unwrap();
    learner.record(&candidate("开发"));
    learner.record_choice("kaifa", "", "开发");
    learner.record_transition(Context::START, "开发", 1);
    learner.record_typo("gan", "guan");
    learner.learn_word("账套", &["zhang".into(), "tao".into()]);
    learner.learn_english("gist");
    assert!(learner.has_unsaved());
    learner.flush();
    assert!(!learner.has_unsaved());
    let names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names.len(), 9, "{names:?}");
    assert!(names.iter().all(|name| !name.contains(".tmp")), "{names:?}");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn missing_file_is_empty_table() {
    let learner = FrequencyLearner::from_path("/nonexistent/qingjian-user.tsv").unwrap();
    assert!(learner.is_empty());
}

#[test]
fn forget_removes_the_user_word_and_every_trace_of_learning() {
    let mut learner = FrequencyLearner::default();
    learner.learn_word("账套", &["zhang".into(), "tao".into()]);
    learner.record(&candidate("账套"));
    learner.record_choice("zhangtao", "", "账套");
    learner.record_choice("zt", "", "账套");
    learner.record_choice("zt", "", "周天");
    learner.record_transition(Context::START, "账套", 1);
    learner.record_transition(Context::after("账套"), "建好", 1);
    let forgotten = learner.forget("账套");
    assert!(forgotten.user_word && forgotten.learning);
    assert!(learner.user_words().is_none());
    assert_eq!(learner.weight("账套"), 0);
    assert_eq!(learner.choice_weight("zt", "账套"), 0);
    assert_eq!(learner.choice_weight("zt", "周天"), 1);
    assert!(learner.user_ngram().is_none());
    // 词库词、没学过：什么都没清
    assert!(learner.forget("开发").is_nothing());
    // 只学过、不是用户词
    learner.record(&candidate("开发"));
    let forgotten = learner.forget("开发");
    assert!(!forgotten.user_word && forgotten.learning);
    // 个人英文词
    learner.learn_english("gist");
    assert!(learner.forget_english("Gist"));
    assert!(!learner.forget_english("gist"));
    assert!(learner.user_english().is_none());
}

#[test]
fn user_words_round_trip_through_tsv_and_form_a_dictionary() {
    let dir = std::env::temp_dir().join("qingjian-user-words-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user.tsv");
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(dir.join(USER_WORDS_FILE));

    let mut learner = FrequencyLearner::from_path(&path).unwrap();
    assert!(learner.user_words().is_none());
    learner.learn_word("账套", &["zhang".to_owned(), "tao".to_owned()]);
    learner.learn_word("账套", &["zhang".to_owned(), "tao".to_owned()]);
    assert_eq!(learner.word_count(), 1);
    let hits = learner
        .user_words()
        .unwrap()
        .lookup(&["zhang", "tao"], false);
    assert_eq!(hits[0].text, "账套");
    learner.flush();

    let reloaded = FrequencyLearner::from_path(&path).unwrap();
    assert_eq!(reloaded.word_count(), 1);
    let hits = reloaded.user_words().unwrap().lookup(&["zhang"], true);
    assert_eq!(hits[0].text, "账套");
}

#[test]
fn english_words_round_trip_through_tsv_and_form_a_word_list() {
    let dir = std::env::temp_dir().join("qingjian-user-english-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user.tsv");
    let _ = std::fs::remove_file(FrequencyLearner::english_path(&path));
    let mut learner = FrequencyLearner::from_path(&path).unwrap();
    assert!(learner.user_english().is_none());
    learner.learn_english("gist");
    learner.learn_english("Gist");
    learner.learn_english("python");
    assert_eq!(learner.english_count(), 2);
    // 第一次敲的写法留下，次数合并
    assert_eq!(learner.user_english().unwrap().get("gist"), Some("gist"));
    assert_eq!(learner.user_english().unwrap().complete("g", 3), ["gist"]);
    learner.flush();
    let reloaded = FrequencyLearner::from_path(&path).unwrap();
    assert_eq!(reloaded.english_count(), 2);
    assert_eq!(
        reloaded.user_english().unwrap().get("python"),
        Some("python")
    );
    let saved = std::fs::read_to_string(FrequencyLearner::english_path(&path)).unwrap();
    assert!(saved.contains("gist\t2"));
}

#[test]
fn weight_decays_by_half_life_and_only_year_cold_entries_are_pruned() {
    let dir = std::env::temp_dir().join("qingjian-decay-test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user.tsv");

    let mut learner = FrequencyLearner::from_path(&path).unwrap();
    learner.record(&candidate("开发"));
    learner.record(&candidate("开发"));
    learner.record(&candidate("中文"));
    // 拨到 90 天前：90 天半衰期下 2 次变 1 次、1 次变 0.5 回 1
    for entry in learner.counts.values_mut() {
        entry.last_used -= 90;
    }
    assert_eq!(learner.weight("开发"), 1);
    assert_eq!(learner.weight("中文"), 1);
    // 冷却到 0 也只压排序：记录留着，重排后仍能查出原次数
    for entry in learner.counts.values_mut() {
        entry.last_used -= 180;
    }
    assert_eq!(learner.weight("开发"), 0);
    assert_eq!(learner.counts.get("开发").unwrap().count, 2, "记录本身不减");
    learner.flush();
    // 冷不到一年的记录原样保留（主表旧格式两列，日期在侧车）
    assert_eq!(learner.counts.len(), 2);
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(saved.contains("开发\t2"), "{saved:?}");
    let days = std::fs::read_to_string(FrequencyLearner::days_path(&path)).unwrap();
    assert!(days.contains("开发\t"), "{days:?}");

    // 拨过一年：落盘时文件与内存一起清出（没有新记录也清，直接走保存）
    for entry in learner.counts.values_mut() {
        entry.last_used -= 400;
    }
    learner.save_to(&path).unwrap();
    assert!(learner.counts.is_empty());
    let days = std::fs::read_to_string(FrequencyLearner::days_path(&path)).unwrap();
    assert!(!days.contains("开发"), "{days:?}");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn days_come_from_the_sidecar_and_fall_back_to_mtime() {
    let mut learner = FrequencyLearner::default();
    // 主表旧格式两列；日期在侧车文件里，没有的条目退回 fallback_day
    let today = learner.today.get();
    let loaded = learner.load_counts("开发\t10\n中文\t4\n", today);
    assert_eq!(loaded, 0);
    learner.load_days(&format!("开发\t{}\n", today - 180));
    assert_eq!(
        learner.counts.get("开发").unwrap().count,
        10,
        "记录本身不减"
    );
    assert_eq!(
        learner.weight("开发"),
        3,
        "180 天前的 10 次折半两次，四舍五入 3"
    );
    assert_eq!(
        learner.weight("中文"),
        4,
        "侧车没提的条目按 fallback_day（今天）算"
    );
    // 旧版本写的主表（没有侧车）照读，整份按文件的修改日计
    let mut learner = FrequencyLearner::default();
    let loaded = learner.load_counts("旧词\t7\n坏行\n", today - 90);
    assert_eq!(loaded, 1);
    assert_eq!(
        learner.weight("旧词"),
        4,
        "90 天前的 7 次衰减到约 3.5，四舍五入 4"
    );
}

#[test]
fn cold_english_words_stay_in_the_list_until_a_year_cold() {
    let dir = std::env::temp_dir().join("qingjian-english-decay-test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user.tsv");

    let mut learner = FrequencyLearner::from_path(&path).unwrap();
    learner.learn_english("stale");
    learner.learn_english("fresh");
    // 冷到 0 的英文词仍在词表里（排序权重最低按 1），别让用户觉得学过的词被忘掉了
    learner.english.get_mut("stale").unwrap().1.last_used -= 200;
    learner.flush();
    assert_eq!(learner.english_count(), 2);
    assert_eq!(learner.user_english().unwrap().get("stale"), Some("stale"));
    let saved = std::fs::read_to_string(FrequencyLearner::english_path(&path)).unwrap();
    assert!(saved.contains("stale\t1"), "{saved:?}");
    // 冷过一年：落盘时清出（没有新记录也清，直接走保存）
    learner.english.get_mut("stale").unwrap().1.last_used -= 600;
    learner
        .save_english_to(&FrequencyLearner::english_path(&path))
        .unwrap();
    assert_eq!(learner.english_count(), 1);
    let saved = std::fs::read_to_string(FrequencyLearner::english_path(&path)).unwrap();
    assert!(!saved.contains("stale"), "{saved:?}");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn choices_generalize_by_reading_key() {
    let mut learner = FrequencyLearner::default();
    // `wod` 下选的 我的：字面键与读音键各记一份
    learner.record_choice("wod", "wode", "我的");
    assert_eq!(learner.choice_weight("wod", "我的"), 1);
    // 换个打法把音节打全：读音键在记录侧已写好，查询按字面键直接命中
    assert_eq!(learner.choice_weight("wode", "我的"), 1);
    // 别的词不受影响
    assert_eq!(learner.choice_weight("wode", "沃德"), 0);
    // 打一半的短查询不带走更长输入下的选择：`yiyi` 选的 意义 不冒到 `yiy` 上
    //（`yiy` 是用户选 一样 的地方，长输入的历史不该隔串压它）
    learner.record_choice("yiyi", "yiyi", "意义");
    assert_eq!(learner.choice_weight("yiy", "意义"), 0);
    // 双字母简拼也泛化到全拼读音（`sm` 选的 上面，`shangmian` 再打认账）
    learner.record_choice("sm", "shangmian", "上面");
    assert_eq!(learner.choice_weight("shangmian", "上面"), 1);
    // 单字母简拼不泛化：`g` 选的 该 不进 `gai` 键（那里是用户认真打 改 的地方）
    learner.record_choice("g", "gai", "该");
    assert_eq!(learner.choice_weight("gai", "该"), 0);
    assert_eq!(learner.choice_weight("g", "该"), 1);
    // 撤销把两个键一起退掉
    learner.unrecord_choice("wod", "wode", "我的");
    assert_eq!(learner.choice_weight("wod", "我的"), 0);
    assert_eq!(learner.choice_weight("wode", "我的"), 0);
    // 与撤销无关的记录还在：意义 / 上面（两条键）/ 该
    assert_eq!(learner.choice_count(), 4);
}
