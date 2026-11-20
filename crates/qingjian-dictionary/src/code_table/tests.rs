//! 码表测试：查询顺序、去重、合并、落盘对拍、错误路径与并发。

use super::*;

use qingjian_format::Metadata;

use crate::matching::Match;

#[test]
fn inserts_phrases_without_changing_shortcut_order_or_duplicates() {
    let mut table = CodeTable::parse("甲\ta\t1\n乙\ta\t100\n甲\tabcd\t10\n").unwrap();
    assert!(table.insert_phrase("甲乙", "abef", 100));
    assert!(!table.insert_phrase("甲乙", "abef", 100));
    assert!(!table.insert_phrase("甲", "x", 100));
    assert_eq!(
        table
            .lookup("a", 3)
            .iter()
            .map(|hit| hit.text)
            .collect::<Vec<_>>(),
        ["甲", "乙", "甲乙"]
    );
    assert_eq!(table.full_code('甲'), Some("abcd"));
    assert_eq!(table.total_frequency(), 211);
}

#[test]
fn looks_up_by_prefix_and_prefers_the_exact_code() {
    let table =
        CodeTable::parse("开发\tgant\t900\n开\tga\t5000\n一\tggll\t100000\n个\twhj\t8000\n")
            .unwrap();
    // 打全了的编码排在前缀命中的词前面，哪怕前缀词词频更高
    let hits = table.lookup("ga", 10);
    assert_eq!(
        hits.iter().map(|h| (h.text, h.exact)).collect::<Vec<_>>(),
        [("开", true), ("开发", false)]
    );
    // 没打全的编码只按词频
    let hits = table.lookup("g", 10);
    assert_eq!(
        hits.iter().map(|h| h.text).collect::<Vec<_>>(),
        ["一", "开", "开发"]
    );
    assert!(hits.iter().all(|h| !h.exact));
    assert!(table.lookup("zzz", 10).is_empty());
    assert!(table.lookup("", 10).is_empty());
    assert!(table.lookup("ga", 0).is_empty());
}

#[test]
fn truncates_to_the_limit_keeping_the_best() {
    // 命中条数多于 limit 时先线性选一段再排，结果要与整体排序的前几条一致
    let table =
        CodeTable::parse("甲\tgaa\t10\n乙\tgab\t50\n丙\tgac\t30\n丁\tgad\t40\n戊\tgae\t20\n")
            .unwrap();
    assert_eq!(
        table
            .lookup("ga", 3)
            .iter()
            .map(|h| h.text)
            .collect::<Vec<_>>(),
        ["乙", "丁", "丙"]
    );
    assert_eq!(table.lookup("ga", 9).len(), 5);
    assert_eq!(table.lookup("ga", 5).len(), 5);
}

#[test]
fn single_letter_codes_follow_table_order_before_frequency() {
    let table = CodeTable::parse("首选\tg\t1\n次选\tg\t999999\n前缀词\tga\t1000000\n第三\tg\t10\n")
        .unwrap();
    assert_eq!(
        table
            .lookup("g", 2)
            .iter()
            .map(|hit| hit.text)
            .collect::<Vec<_>>(),
        ["首选", "次选"]
    );
    assert_eq!(
        table
            .lookup("g", 10)
            .iter()
            .map(|hit| hit.text)
            .collect::<Vec<_>>(),
        ["首选", "次选", "第三", "前缀词"]
    );
}

#[test]
fn keeps_the_highest_frequency_of_a_duplicated_entry() {
    let table = CodeTable::parse("开发\tgant\t900\n开发\tgant\t3000\n").unwrap();
    assert_eq!(table.len(), 1);
    assert_eq!(table.lookup("gant", 10)[0].frequency, 3000);
}

#[test]
fn merges_user_words_into_the_table() {
    let mut table = CodeTable::parse("一\tggll\t100000\n开发\tgant\t900\n").unwrap();
    table.merge(CodeTable::parse("青简\tgetu\t1\n开发\tgant\t5\n小青\tihge\t1\n").unwrap());
    assert_eq!(table.len(), 4);
    assert_eq!(table.lookup("getu", 10)[0].text, "青简");
    assert_eq!(table.lookup("gant", 10)[0].frequency, 900);
    assert_eq!(table.total_frequency(), 100000 + 900 + 1 + 1);
    assert_eq!(table.full_code_len(), 4);
}

#[test]
fn merged_single_letter_codes_follow_the_combined_table_order() {
    let mut table = CodeTable::parse("主表\tg\t1\n").unwrap();
    table.merge(CodeTable::parse("追加\tg\t999999\n").unwrap());
    assert_eq!(
        table
            .lookup("g", 10)
            .iter()
            .map(|hit| hit.text)
            .collect::<Vec<_>>(),
        ["主表", "追加"]
    );
}

#[test]
fn rejects_lines_without_a_code() {
    assert!(CodeTable::parse("开发\n").is_err());
    assert!(CodeTable::parse("开发\tgant\t不是数字\n").is_err());
}

#[test]
fn skips_codes_that_are_not_letters() {
    let table = CodeTable::parse("开\tga\t5000\n逗\tg;\t10\n").unwrap();
    assert_eq!(table.len(), 1);
}

#[test]
fn the_full_code_is_the_most_common_length_not_the_longest() {
    let table =
        CodeTable::parse("开\tga\t5000\n一\tggll\t100000\n中国\tkhlg\t9000\nTF卡\tttfhh\t1\n")
            .unwrap();
    assert_eq!(table.full_code_len(), 4);
    let table = CodeTable::parse("开\tga\t5000\n一\tggll\t100000\n").unwrap();
    assert_eq!(table.full_code_len(), 4);
    assert_eq!(CodeTable::default().full_code_len(), 0);
}

#[test]
fn normalizes_codes_to_lowercase() {
    let table = CodeTable::parse("开\tGA\t5000\n").unwrap();
    assert_eq!(table.lookup("ga", 10)[0].text, "开");
}

/// 随包五笔是 `.qj`：打包后单码的行序、词频与满码长度都不变。
#[test]
fn qj_round_trip_keeps_order_and_frequency() {
    let dir = std::env::temp_dir().join(format!("qingjian-code-table-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("wubi86.qj");
    let table =
        CodeTable::parse("工\ta\t100\n戈\ta\t900\n开发\tgant\t9000\n一\tggll\t50\n").unwrap();
    table.write_qj(&path, &Metadata::default()).unwrap();
    let packed = CodeTable::from_path(&path).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    let texts = |table: &CodeTable, code: &str| {
        table
            .lookup(code, 5)
            .into_iter()
            .map(|m| m.text.to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(texts(&packed, "a"), texts(&table, "a"));
    assert_eq!(texts(&packed, "gan"), ["开发"]);
    assert_eq!(packed.total_frequency(), table.total_frequency());
    assert_eq!(packed.full_code_len(), table.full_code_len());
}

fn lcg(mut state: u64) -> impl FnMut(usize) -> usize {
    move |bound| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((state >> 33) as usize) % bound
    }
}

fn snapshot(table: &CodeTable) -> Vec<(&str, &str, u32)> {
    let mut rows: Vec<(&str, &str, u32)> = table.entries_view().collect();
    rows.sort();
    rows
}

/// 随机条目（含非 ASCII、空格）落盘再打开：条目、词频、满码长度与前缀查询一致。
#[test]
fn qj_round_trip_preserves_random_entries() {
    let mut next = lcg(0x517C_C1B7_2722_0A95);
    let text_pool = [
        "开发",
        "手机",
        "café",
        "tiếng Việt",
        "a b",
        "ü",
        "电脑",
        "上海",
    ];
    let letters = ['a', 'b', 'c', 'f', 'g', 'h', 'k'];
    let mut tsv = String::new();
    for _ in 0..60 {
        let code_len = 1 + next(4);
        let code: String = (0..code_len)
            .map(|_| letters[next(letters.len())])
            .collect();
        let text = text_pool[next(text_pool.len())];
        let frequency = 1 + next(500);
        tsv.push_str(&format!("{text}\t{code}\t{frequency}\n"));
    }
    let original = CodeTable::parse(&tsv).unwrap();

    let dir =
        std::env::temp_dir().join(format!("qingjian-code-table-random-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("random.qj");
    original.write_qj(&path, &Metadata::default()).unwrap();
    let reopened = CodeTable::from_path(&path).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();

    assert_eq!(reopened.len(), original.len());
    assert_eq!(reopened.total_frequency(), original.total_frequency());
    assert_eq!(reopened.full_code_len(), original.full_code_len());
    assert_eq!(snapshot(&reopened), snapshot(&original));
    for prefix in ["a", "g", "ka", "zzz"] {
        assert_eq!(
            reopened
                .lookup(prefix, 20)
                .iter()
                .map(|m| m.text)
                .collect::<Vec<_>>(),
            original
                .lookup(prefix, 20)
                .iter()
                .map(|m| m.text)
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn missing_file_and_magic_corruption_are_rejected() {
    let dir = std::env::temp_dir().join(format!("qingjian-code-table-err-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let missing = dir.join("missing.qj");
    assert!(CodeTable::from_path(&missing).is_err());

    // 魔数对得上但头是坏的：走 `.qj` 容器路径并报错，而不是静默当 TSV 读。
    let corrupt = dir.join("corrupt.qj");
    let mut bytes = qingjian_format::MAGIC.to_vec();
    bytes.extend(std::iter::repeat_n(0u8, 64));
    std::fs::write(&corrupt, &bytes).unwrap();
    assert!(CodeTable::from_path(&corrupt).is_err());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn empty_query_and_unknown_code_return_nothing() {
    let table = CodeTable::parse("开发\tgant\t900\n开\tga\t5000\n").unwrap();
    assert!(table.lookup("", 10).is_empty());
    assert!(table.lookup("ga", 0).is_empty());
    assert!(table.lookup("zzzzzzzzzzzz", 10).is_empty());
    assert!(table.lookup("gant", 10)[0].exact);
}

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn code_table_is_send_sync_and_concurrent_lookups_are_stable() {
    assert_send_sync::<CodeTable>();
    let table = std::sync::Arc::new(
        CodeTable::parse("开发\tgant\t900\n开\tga\t5000\n一\tggll\t100000\n").unwrap(),
    );
    let owned = |hits: &[Match<'_>]| {
        let mut rows: Vec<(String, String, u32, bool)> = hits
            .iter()
            .map(|m| (m.text.to_owned(), m.pinyin.to_owned(), m.frequency, m.exact))
            .collect();
        rows.sort();
        rows
    };
    let baseline = owned(&table.lookup("g", 10));
    let mut handles = Vec::new();
    for _ in 0..8 {
        let table = std::sync::Arc::clone(&table);
        handles.push(std::thread::spawn(move || {
            (0..50)
                .map(|_| owned(&table.lookup("g", 10)))
                .collect::<Vec<_>>()
        }));
    }
    for handle in handles {
        for hits in handle.join().unwrap() {
            assert_eq!(hits, baseline);
        }
    }
}
