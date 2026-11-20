//! 查询契约 golden 断言，以及收窄路径与线性扫描的随机对拍。

use super::*;

use crate::pattern::SyllablePattern;

/// 查询命中集合、顺序、`exact` 标记与空/未知/超长输入的契约，用固定小词库锁死。
#[test]
fn query_contract_golden_locks_set_order_and_flags() {
    let dictionary = Dictionary::parse(GOLDEN).unwrap();

    // 完整两音节：精确命中在前，以它为前缀的更长词在后。
    let matches = dictionary.lookup(&["kai", "fa"], false);
    assert_eq!(texts(&matches), ["开发", "开发者"]);
    assert!(matches[0].exact);
    assert!(!matches[1].exact);
    assert_eq!(matches[0].pinyin, "kai fa");
    assert_eq!(matches[1].pinyin, "kai fa zhe");

    // 完整音节的普通查询同样收以它为前缀的更长词；精确词排第一，`exact` 标记区分。
    let complete = dictionary.lookup(&["kai"], false);
    assert_eq!(texts(&complete), ["开", "开发", "开发者", "开饭", "开放"]);
    assert!(complete[0].exact);
    assert!(complete[1..].iter().all(|m| !m.exact));
    // 只要音节数恰好相等的词，走 `lookup_exact`。
    assert_eq!(
        texts(&dictionary.lookup_exact(&[SyllablePattern::complete("kai")])),
        ["开"]
    );

    // 最后一个音节按前缀：同前缀的单字与长短词全收，精确标记按音节数区分。
    let partial = dictionary.lookup(&["kai"], true);
    assert_eq!(texts(&partial), ["开", "开发", "开发者", "开饭", "开放"]);
    assert!(partial[0].exact);
    assert!(partial[1..].iter().all(|m| !m.exact));

    // 完整音节 `ka` 不等于 `kai`，只命中 `ka fei`。
    let ka = dictionary.lookup(&["ka"], false);
    assert_eq!(texts(&ka), ["咖啡"]);
    assert!(!ka[0].exact);

    // 其他键互不串扰。
    assert_eq!(texts(&dictionary.lookup(&["shou"], false)), ["手机"]);
    assert_eq!(texts(&dictionary.lookup(&["ji"], false)), ["计算"]);

    // 空查询：空序列与空写法切片为空结果；空音节串按调用契约不会传进来。
    assert!(dictionary.lookup(&[], false).is_empty());
    assert!(dictionary.lookup_pattern(&[]).is_empty());
    assert!(
        dictionary
            .lookup_pattern_alt(&[Vec::<SyllablePattern<'_>>::new()])
            .is_empty()
    );

    // 未知编码与超长编码：空结果。
    assert!(dictionary.lookup(&["zzz"], false).is_empty());
    assert!(dictionary.lookup(&["kai", "zzz"], false).is_empty());
    assert!(
        dictionary
            .lookup(&["pneumonoultramicroscopicsilicovolcanoconiosis"], false)
            .is_empty()
    );

    // 互不覆盖的多种写法不产生重复命中。
    let alt_positions: Vec<Vec<SyllablePattern<'_>>> = vec![
        vec![SyllablePattern::complete("kai")],
        vec![
            SyllablePattern::complete("fan"),
            SyllablePattern::complete("fang"),
        ],
    ];
    let alt = dictionary.lookup_pattern_alt(&alt_positions);
    let mut seen = std::collections::HashSet::new();
    assert!(alt.iter().all(|m| seen.insert((m.text, m.pinyin))));
    assert_eq!(texts(&alt), ["开饭", "开放"]);

    // 简拼两字母：命中集合是 k 开头 + 第二音节 f 开头的全部词。
    let initials =
        dictionary.lookup_pattern(&[SyllablePattern::prefix("k"), SyllablePattern::prefix("f")]);
    let mut found = texts(&initials);
    found.sort_unstable();
    assert_eq!(found, ["咖啡", "开发", "开发者", "开放", "开饭"]);
}

const PROPERTY_SYLLABLES: [&str; 14] = [
    "a", "an", "ang", "ka", "kai", "kan", "kang", "fa", "fan", "fang", "fei", "shi", "sha", "shang",
];

/// 多份随机词库 × 随机查询（前缀 / 完全匹配 / 不匹配）：收窄路径与线性扫描结果集合必须一致。
#[test]
fn narrowing_agrees_with_linear_scan_on_random_tables() {
    let mut next = lcg(0x2545_F491_4F6C_DD1D);
    let by_key = |a: &Match<'_>, b: &Match<'_>| (a.pinyin, a.text).cmp(&(b.pinyin, b.text));
    let mut non_empty = 0usize;
    let mut rounds = 0usize;

    for round in 0..220 {
        // 每份词库用池子的一个随机子集，词条数与键长随机。
        let pool_size = 4 + next(PROPERTY_SYLLABLES.len() - 3);
        let pool: Vec<&str> = (0..pool_size)
            .map(|_| PROPERTY_SYLLABLES[next(PROPERTY_SYLLABLES.len())])
            .collect();
        let entry_count = 30 + next(90);
        let mut tsv = String::new();
        for i in 0..entry_count {
            let count = 1 + next(3);
            let key: Vec<&str> = (0..count).map(|_| pool[next(pool.len())]).collect();
            tsv.push_str(&format!("词{i}\t{}\t{}\n", key.join(" "), 1 + next(500)));
        }
        let dictionary = Dictionary::parse(&tsv).unwrap();

        // 随机查询：大部分取自池子（完全 / 前缀各半），小半是池外编码（不匹配）。
        let position_count = 1 + next(2);
        let positions: Vec<Vec<SyllablePattern<'_>>> = (0..position_count)
            .map(|_| {
                if next(10) < 3 {
                    // 不匹配：池外编码。
                    let unknown = ["zz", "qx", "qz", "xx"][next(4)];
                    vec![SyllablePattern::complete(unknown)]
                } else {
                    let syllable = pool[next(pool.len())];
                    if next(2) == 0 {
                        vec![SyllablePattern::complete(syllable)]
                    } else {
                        vec![SyllablePattern::prefix(
                            &syllable[..1 + next(syllable.len())],
                        )]
                    }
                }
            })
            .collect();

        let single: Vec<SyllablePattern<'_>> = positions.iter().map(|p| p[0]).collect();
        let mut slow = lookup_scan(&dictionary, &positions);
        slow.sort_by(by_key);
        let mut fast = dictionary.lookup_pattern(&single);
        fast.sort_by(by_key);
        assert_eq!(fast, slow, "round {round} positions {positions:?}");

        let mut slow_exact: Vec<_> = slow.iter().copied().filter(|m| m.exact).collect();
        slow_exact.sort_by(by_key);
        let mut fast_exact = dictionary.lookup_exact(&single);
        fast_exact.sort_by(by_key);
        assert_eq!(
            fast_exact, slow_exact,
            "exact round {round} positions {positions:?}"
        );

        non_empty += usize::from(!fast.is_empty());
        rounds += 1;
    }
    assert_eq!(rounds, 220);
    assert!(non_empty > 80, "too few non-empty cases: {non_empty}");
}
