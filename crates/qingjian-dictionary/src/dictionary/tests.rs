use super::*;

const SAMPLE: &str = "# 测试词库\n开发\tkai fa\t9000\n开发者\tkai fa zhe\t3000\n开饭\tkai fan\t800\n开放\tkai fang\t2000\n咖啡\tka fei\t5000\n开\tkai\t20000\n";

fn texts<'a>(matches: &'a [Match<'a>]) -> Vec<&'a str> {
    matches.iter().map(|m| m.text).collect()
}

#[test]
fn full_syllables_match_exact_and_longer_words() {
    let dictionary = Dictionary::parse(SAMPLE).unwrap();
    let matches = dictionary.lookup(&["kai", "fa"], false);
    assert_eq!(texts(&matches), ["开发", "开发者"]);
    assert!(matches[0].exact);
    assert!(!matches[1].exact);
    assert_eq!(matches[0].pinyin, "kai fa");
}

#[test]
fn exact_lookup_only_returns_words_of_the_same_length() {
    let dictionary = Dictionary::parse(SAMPLE).unwrap();
    let exact = dictionary.lookup_exact(&[
        SyllablePattern::complete("kai"),
        SyllablePattern::complete("fa"),
    ]);
    assert_eq!(texts(&exact), ["开发"]);
    let partial = dictionary.lookup_exact(&[
        SyllablePattern::complete("kai"),
        SyllablePattern::prefix("f"),
    ]);
    assert_eq!(texts(&partial), ["开发", "开饭", "开放"]);
    let single = dictionary.lookup_exact(&[SyllablePattern::complete("kai")]);
    assert_eq!(texts(&single), ["开"]);
    assert!(
        dictionary
            .lookup_exact(&[SyllablePattern::complete("ka")])
            .is_empty()
    );
    assert_eq!(
        dictionary.total_frequency(),
        9000 + 3000 + 800 + 2000 + 5000 + 20000
    );
}

#[test]
fn partial_last_syllable_matches_by_prefix() {
    let dictionary = Dictionary::parse(SAMPLE).unwrap();
    let matches = dictionary.lookup(&["kai", "fa"], true);
    assert_eq!(texts(&matches), ["开发", "开发者", "开饭", "开放"]);
}

#[test]
fn single_complete_syllable_does_not_match_longer_syllables() {
    let dictionary = Dictionary::parse(SAMPLE).unwrap();
    let matches = dictionary.lookup(&["ka"], false);
    assert_eq!(texts(&matches), ["咖啡"]);
    assert!(!matches[0].exact);
}

#[test]
fn initials_pattern_matches_abbreviations() {
    let dictionary = Dictionary::parse(SAMPLE).unwrap();
    let matches =
        dictionary.lookup_pattern(&[SyllablePattern::prefix("k"), SyllablePattern::prefix("f")]);
    let mut found = texts(&matches);
    found.sort_unstable();
    assert_eq!(found, ["咖啡", "开发", "开发者", "开放", "开饭"]);
    let mixed = dictionary.lookup_pattern(&[
        SyllablePattern::complete("kai"),
        SyllablePattern::prefix("f"),
    ]);
    assert!(!texts(&mixed).contains(&"咖啡"));
}

#[test]
fn homophones_are_sorted_by_frequency() {
    let dictionary = Dictionary::parse("发\tfa\t100\n法\tfa\t300\n乏\tfa\t50\n").unwrap();
    assert_eq!(
        texts(&dictionary.lookup(&["fa"], false)),
        ["法", "发", "乏"]
    );
    assert_eq!(dictionary.len(), 3);
}

#[test]
fn normalizes_u_umlaut_keys_and_queries() {
    let dictionary = Dictionary::parse("略\tlue\t100\n虐\tnve\t90\n").unwrap();
    assert_eq!(dictionary.entries().next().unwrap().pinyin, "lve");
    assert_eq!(texts(&dictionary.lookup(&["lue"], false)), ["略"]);
    assert_eq!(texts(&dictionary.lookup(&["lve"], false)), ["略"]);
    assert_eq!(texts(&dictionary.lookup(&["nue"], false)), ["虐"]);
    assert_eq!(texts(&dictionary.lookup(&["nve"], false)), ["虐"]);
}

#[test]
fn rejects_malformed_line() {
    let error = Dictionary::parse("开发\tkai fa\tabc\n").unwrap_err();
    assert!(matches!(error, DictionaryError::Line { line: 1, .. }));
}

/// 与线性扫描版对拍：随机词库、随机模式（含每个位置多种写法），结果集必须完全一致（顺序按键排序后比较）。
#[test]
fn narrowing_agrees_with_linear_scan() {
    const SYLLABLES: [&str; 14] = [
        "a", "an", "ang", "ka", "kai", "kan", "kang", "fa", "fan", "fang", "fei", "shi", "sha",
        "shang",
    ];
    // 简单的线性同余随机数，测试可复现
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move |bound: usize| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((state >> 33) as usize) % bound
    };
    let mut tsv = String::new();
    for i in 0..3000 {
        let count = 1 + next(4);
        let key: Vec<&str> = (0..count)
            .map(|_| SYLLABLES[next(SYLLABLES.len())])
            .collect();
        tsv.push_str(&format!("词{i}\t{}\t{}\n", key.join(" "), 1 + next(1000)));
    }
    let dictionary = Dictionary::parse(&tsv).unwrap();
    let random_pattern = |next: &mut dyn FnMut(usize) -> usize| {
        let syllable = SYLLABLES[next(SYLLABLES.len())];
        if next(2) == 0 {
            SyllablePattern::complete(syllable)
        } else {
            SyllablePattern::prefix(&syllable[..1 + next(syllable.len())])
        }
    };
    let by_key = |a: &Match<'_>, b: &Match<'_>| (a.pinyin, a.text).cmp(&(b.pinyin, b.text));
    let mut cases = 0;
    for round in 0..600 {
        let count = 1 + next(4);
        let positions: Vec<Vec<SyllablePattern<'_>>> = (0..count)
            .map(|_| {
                let alternatives = if round < 300 { 1 } else { 1 + next(3) };
                let mut position: Vec<SyllablePattern<'_>> = Vec::new();
                for _ in 0..alternatives {
                    let candidate = random_pattern(&mut next);
                    // 契约：同一位置的写法不能互相覆盖
                    let overlaps = position.iter().any(|p| {
                        (!p.complete && candidate.text.starts_with(p.text))
                            || (!candidate.complete && p.text.starts_with(candidate.text))
                            || p.text == candidate.text
                    });
                    if !overlaps {
                        position.push(candidate);
                    }
                }
                position
            })
            .collect();
        let mut slow = dictionary.lookup_scan(&positions);
        slow.sort_by(by_key);
        let mut fast = if round < 300 {
            let single: Vec<SyllablePattern<'_>> = positions.iter().map(|p| p[0]).collect();
            dictionary.lookup_pattern(&single)
        } else {
            dictionary.lookup_pattern_alt(&positions)
        };
        fast.sort_by(by_key);
        assert_eq!(fast, slow, "positions {positions:?}");
        let mut exact = if round < 300 {
            let single: Vec<SyllablePattern<'_>> = positions.iter().map(|p| p[0]).collect();
            dictionary.lookup_exact(&single)
        } else {
            dictionary.lookup_exact_alt(&positions)
        };
        exact.sort_by(by_key);
        slow.retain(|m| m.exact);
        assert_eq!(exact, slow, "exact positions {positions:?}");
        cases += usize::from(!fast.is_empty());
    }
    assert!(cases > 200, "too few non-empty cases: {cases}");
}

#[test]
fn qj_round_trip_preserves_every_lookup() {
    let dictionary = Dictionary::parse(SAMPLE).unwrap();
    let dir = std::env::temp_dir().join("qingjian-dictionary-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("dict-{}.qj", std::process::id()));
    let metadata = Metadata {
        name: "测试词库".to_owned(),
        license: "MIT".to_owned(),
        ..Metadata::default()
    };
    dictionary.write_qj(&path, &metadata).unwrap();
    let mapped = Dictionary::from_path(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert!(matches!(&mapped.texts, Text::Mapped { .. }));
    assert!(matches!(&mapped.keys, Text::Mapped { .. }));
    assert!(matches!(&mapped.index, Table::Mapped { .. }));
    assert!(matches!(&mapped.slots, Table::Mapped { .. }));
    assert_eq!(mapped.len(), dictionary.len());
    assert_eq!(mapped.total_frequency(), dictionary.total_frequency());
    assert_eq!(mapped.metadata().unwrap().name, "测试词库");
    assert_eq!(mapped.metadata().unwrap().entries, dictionary.len() as u64);
    for (syllables, partial) in [
        (vec!["kai", "fa"], false),
        (vec!["kai"], true),
        (vec!["k", "f"], true),
        (vec!["ka"], true),
    ] {
        let original = dictionary.lookup(&syllables, partial);
        let reopened = mapped.lookup(&syllables, partial);
        assert_eq!(texts(&reopened), texts(&original), "{syllables:?}");
    }
}

#[test]
fn legacy_qj_umlaut_keys_remain_queryable() {
    let legacy = Dictionary {
        texts: Text::Owned("略虐".to_owned()),
        keys: Text::Owned("luenue".to_owned()),
        index: Table::Owned(vec![
            KeyIndex {
                key_start: 0,
                first_slot: 0,
                slot_count: 1,
                key_len: 3,
                reserved: 0,
            },
            KeyIndex {
                key_start: 3,
                first_slot: 1,
                slot_count: 1,
                key_len: 3,
                reserved: 0,
            },
        ]),
        slots: Table::Owned(vec![
            Slot {
                text_start: 0,
                frequency: 100,
                text_len: 3,
                reserved: 0,
            },
            Slot {
                text_start: 3,
                frequency: 90,
                text_len: 3,
                reserved: 0,
            },
        ]),
        total_frequency: 190,
        metadata: None,
    };
    let dir = std::env::temp_dir().join("qingjian-dictionary-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("legacy-dict-{}.qj", std::process::id()));
    let metadata = Metadata {
        name: "旧词库".to_owned(),
        ..Metadata::default()
    };
    legacy.write_qj(&path, &metadata).unwrap();

    let dictionary = Dictionary::open_qj(&path).unwrap();
    std::fs::remove_file(&path).unwrap();

    assert!(matches!(&dictionary.texts, Text::Owned(_)));
    assert!(matches!(&dictionary.keys, Text::Owned(_)));
    assert!(matches!(&dictionary.index, Table::Owned(_)));
    assert!(matches!(&dictionary.slots, Table::Owned(_)));
    assert_eq!(dictionary.metadata().unwrap().name, "旧词库");
    assert_eq!(texts(&dictionary.lookup(&["lue"], false)), ["略"]);
    assert_eq!(texts(&dictionary.lookup(&["lve"], false)), ["略"]);
    assert_eq!(texts(&dictionary.lookup(&["nue"], false)), ["虐"]);
    assert_eq!(texts(&dictionary.lookup(&["nve"], false)), ["虐"]);
}
