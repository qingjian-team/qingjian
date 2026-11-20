//! 序列化 → 反解析 roundtrip：随机条目（含非 ASCII 与空格）落盘再打开，值一致。

use super::*;

/// 随机条目（含非 ASCII、空格）落盘再打开：条目、词频、声调与前缀查询一致。
#[test]
fn qj_round_trip_preserves_random_entries_with_unicode() {
    let mut next = lcg(0x9E37_79B9_7F4A_7C15);
    let text_pool = [
        "你好",
        "绿色",
        "咖啡",
        "tiếng Việt",
        "café",
        "a b",
        "ü",
        "苹果",
        "电脑",
        "上海",
    ];
    let syllables = [
        "ni", "hao", "lv", "se", "ka", "fei", "shou", "ji", "ping", "guo", "dian", "nao", "shang",
        "hai",
    ];
    let mut tsv = String::new();
    for _ in 0..40 {
        let count = 1 + next(4);
        let key: Vec<String> = (0..count)
            .map(|_| {
                let syllable = syllables[next(syllables.len())];
                if next(2) == 0 {
                    format!("{syllable}{}", 1 + next(5))
                } else {
                    syllable.to_owned()
                }
            })
            .collect();
        let text = text_pool[next(text_pool.len())];
        let frequency = 1 + next(500);
        tsv.push_str(&format!("{text}\t{}\t{frequency}\n", key.join(" ")));
    }
    let original = Dictionary::parse(&tsv).unwrap();

    let path = temp_path("random-roundtrip");
    original.write_qj(&path, &Metadata::default()).unwrap();
    let reopened = Dictionary::open_qj(&path).unwrap();
    std::fs::remove_file(&path).unwrap();

    assert_eq!(reopened.len(), original.len());
    assert_eq!(reopened.total_frequency(), original.total_frequency());
    assert_eq!(snapshot(&reopened), snapshot(&original));
    for query in [&["kai", "fa"][..], &["shou"][..], &["k", "f"][..]] {
        assert_eq!(
            texts(&reopened.lookup(query, true)),
            texts(&original.lookup(query, true))
        );
    }
}
