//! 词级排序：结构项、上下文得分、去重，以及前缀词与单字的先后。

use super::*;
use qingjian_dictionary::Match;

fn hit<'a>(text: &'a str, pinyin: &'a str, frequency: u32, exact: bool) -> Match<'a> {
    Match {
        text,
        pinyin,
        frequency,
        reading_total: 0,
        tones: Default::default(),
        secondary: false,
        exact,
    }
}

#[test]
fn exact_beats_frequency_and_weight_beats_static_frequency() {
    let mut items = vec![
        Scored {
            hit: hit("开发者", "kai fa zhe", 99999, false),
            full_last: true,
            coverage: 5,
            abbreviated: 0,
            weight: 0,
            penalty: 0.0,
            tone_cost: 0,
            split: false,
        },
        Scored {
            hit: hit("开放", "kai fang", 20000, true),
            full_last: false,
            coverage: 5,
            abbreviated: 0,
            weight: 5,
            penalty: 0.0,
            tone_cost: 0,
            split: false,
        },
        Scored {
            hit: hit("开发", "kai fa", 9000, true),
            full_last: true,
            coverage: 5,
            abbreviated: 0,
            weight: 0,
            penalty: 0.0,
            tone_cost: 0,
            split: false,
        },
    ];
    rank(&mut items, usize::MAX, |_| (0, 0.0));
    let texts: Vec<&str> = items.iter().map(|s| s.hit.text).collect();
    assert_eq!(texts, ["开发", "开放", "开发者"]);
}

#[test]
fn context_score_orders_within_the_same_structure() {
    let mut items = vec![
        Scored {
            hit: hit("把", "ba", 3_000_000, true),
            full_last: true,
            coverage: 2,
            abbreviated: 0,
            weight: 0,
            penalty: 0.0,
            tone_cost: 0,
            split: false,
        },
        Scored {
            hit: hit("吧", "ba", 2_000_000, true),
            full_last: true,
            coverage: 2,
            abbreviated: 0,
            weight: 0,
            penalty: 0.0,
            tone_cost: 0,
            split: false,
        },
    ];
    // 上下文说 吧 更像：词频高的 把 让位
    let by_context = |s: &Scored<'_>| (0, if s.hit.text == "吧" { -1.0 } else { -6.0 });
    rank(&mut items, usize::MAX, by_context);
    let texts: Vec<&str> = items.iter().map(|s| s.hit.text).collect();
    assert_eq!(texts, ["吧", "把"]);
    // 同一输入串下选过的压过上下文
    rank(&mut items, usize::MAX, |s| {
        (
            u32::from(s.hit.text == "把"),
            if s.hit.text == "吧" { -1.0 } else { -6.0 },
        )
    });
    assert_eq!(items[0].hit.text, "把");
    // 同分时用户选过的、非模糊音的靠前
    for item in &mut items {
        item.weight = u32::from(item.hit.text == "把") * 3;
    }
    rank(&mut items, usize::MAX, |_| (0, -2.0));
    assert_eq!(items[0].hit.text, "把");
    for item in &mut items {
        item.weight = 3;
        item.penalty = if item.hit.text == "把" {
            FUZZY_PENALTY
        } else {
            0.0
        };
    }
    rank(&mut items, usize::MAX, |_| (0, -2.0));
    assert_eq!(items[0].hit.text, "吧");
}

#[test]
fn fewer_abbreviated_syllables_win_at_equal_coverage() {
    let mut items = vec![
        Scored {
            hit: hit("开放啊", "kai fang a", 132, true),
            full_last: true,
            coverage: 5,
            abbreviated: 1,
            weight: 0,
            penalty: 0.0,
            tone_cost: 0,
            split: false,
        },
        Scored {
            hit: hit("开放", "kai fang", 500_000, true),
            full_last: false,
            coverage: 5,
            abbreviated: 0,
            weight: 0,
            penalty: 0.0,
            tone_cost: 0,
            split: false,
        },
    ];
    rank(&mut items, usize::MAX, |_| (0, 0.0));
    assert_eq!(items[0].hit.text, "开放");
}

#[test]
fn weight_bonus_is_logarithmic_and_capped() {
    assert_eq!(weight_bonus(0), 0.0);
    assert!(weight_bonus(1) < weight_bonus(10));
    assert_eq!(weight_bonus(WEIGHT_CAP), weight_bonus(WEIGHT_CAP * 50));
    assert!(weight_bonus(u32::MAX) < 1.6);
}

#[test]
fn deduplicates_by_text_keeping_best() {
    let mut items = vec![
        Scored {
            hit: hit("西安", "xi an", 4000, false),
            full_last: true,
            coverage: 4,
            abbreviated: 0,
            weight: 0,
            penalty: 0.0,
            tone_cost: 0,
            split: false,
        },
        Scored {
            hit: hit("西安", "xi an", 4000, true),
            full_last: true,
            coverage: 4,
            abbreviated: 0,
            weight: 0,
            penalty: 0.0,
            tone_cost: 0,
            split: false,
        },
    ];
    rank(&mut items, usize::MAX, |_| (0, 0.0));
    assert_eq!(items.len(), 1);
    assert!(items[0].hit.exact);
}

fn candidate<'a>(text: &'a str, coverage: usize, secondary: bool) -> Scored<'a> {
    Scored {
        hit: Match {
            secondary,
            ..hit(text, "", 100, true)
        },
        full_last: true,
        coverage,
        abbreviated: 0,
        weight: 0,
        penalty: 0.0,
        tone_cost: 0,
        split: false,
    }
}

#[test]
fn a_few_words_then_single_chars_and_secondary_prefix_words_last() {
    // `rizhi`：整段 5 个字母，首音节 ri 两个；日职 来自补充词库，盖住整段照样占名额，你邮 这类补充词库的前缀词排最后
    let mut items = vec![
        candidate("日至", 5, false),
        candidate("日治", 5, false),
        candidate("日止", 5, false),
        candidate("日志", 5, false),
        candidate("日职", 5, true),
        candidate("日值", 5, false),
        candidate("日职业", 3, true),
        candidate("日制", 5, false),
        candidate("日", 2, false),
        candidate("驲", 2, false),
    ];
    arrange(&mut items, 5, 5, 2, true, |_| false, |_| false);
    let texts: Vec<&str> = items.iter().map(|s| s.hit.text).collect();
    assert_eq!(
        texts,
        [
            "日至",
            "日治",
            "日止",
            "日志",
            "日值",
            "日制",
            "日",
            "驲",
            "日职",
            "日职业"
        ]
    );
}

#[test]
fn a_single_syllable_keeps_the_ranked_order() {
    let mut items = vec![
        candidate("你", 2, false),
        candidate("尼", 2, true),
        candidate("妮", 2, false),
    ];
    arrange(&mut items, 2, 2, 2, true, |_| false, |_| false);
    let texts: Vec<&str> = items.iter().map(|s| s.hit.text).collect();
    assert_eq!(texts, ["你", "尼", "妮"]);
}

#[test]
fn a_single_syllable_puts_fuzzy_chars_after_the_first_typed_chars() {
    let fuzzy = |text| Scored {
        penalty: FUZZY_PENALTY,
        ..candidate(text, 2, false)
    };
    let typed = ["子", "字", "自", "兹", "仔", "紫", "姊", "资", "滋"];
    let mut items = vec![fuzzy("只"), fuzzy("之"), fuzzy("知")];
    items.extend(typed.iter().map(|text| candidate(text, 2, false)));
    items.push(candidate("孖", 2, false));
    // 这个输入串下选过的 知 不等
    arrange(
        &mut items,
        2,
        2,
        2,
        true,
        |item| item.hit.text == "知",
        |_| false,
    );
    let texts: Vec<&str> = items.iter().map(|s| s.hit.text).collect();
    let mut expected = vec!["知"];
    expected.extend(typed);
    expected.extend(["只", "之", "孖"]);
    assert_eq!(texts, expected);
}
