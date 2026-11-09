//! chaizi 子命令的解析与合并规则测试。

use std::collections::HashMap;

use crate::chaizi::{Simplified, split_of};

fn readings(pairs: &[(&str, &[(&str, u64)])]) -> HashMap<char, Vec<(String, u64)>> {
    pairs
        .iter()
        .map(|(ch, list)| {
            (
                ch.chars().next().unwrap(),
                list.iter()
                    .map(|(syllable, count)| ((*syllable).to_owned(), *count))
                    .collect(),
            )
        })
        .collect()
}

/// 部件读音列与部件列一一对应；部件之间 `'`，同一部件的多个读音用 `,`。
#[test]
fn split_keeps_parts_and_their_readings_apart() {
    let table = readings(&[
        ("木", &[("mu", 100)]),
        (
            "行",
            &[("xing", 90), ("hang", 10), ("heng", 5), ("xing2", 1)],
        ),
        ("口", &[("kou", 80)]),
    ]);
    let one = split_of("木 行", &table, false).unwrap();
    assert_eq!(one.parts_text(), "木行");
    // 一个部件最多留三个读音，按词频
    assert_eq!(one.readings_text(), "mu'xing,hang,heng");
    let two = split_of("木 口", &table, false).unwrap();
    assert_eq!(two.readings_text(), "mu'kou");
}

/// 读不出音的部件留空串，但部件本身照样留着——Core 用它的笔顺顶上。
#[test]
fn unreadable_part_keeps_an_empty_slot() {
    let table = readings(&[("囗", &[]), ("木", &[("mu", 100)])]);
    let one = split_of("囗 木", &table, false).unwrap();
    assert_eq!(one.parts_text(), "囗木");
    assert_eq!(one.readings_text(), "'mu");
    // 空间量对齐：有几个部件就有几段
    assert_eq!(one.readings_text().split('\'').count(), 2);
}

/// 没有读音的部件照样留着（Core 用它的笔顺顶上）；空拆法才丢掉。
#[test]
fn rejects_splits_without_known_parts() {
    let table = readings(&[("木", &[("mu", 100)])]);
    assert_eq!(split_of("木", &table, false).unwrap().parts_text(), "木");
    let bare = split_of("㇆", &table, false).unwrap();
    assert_eq!(bare.parts_text(), "㇆");
    assert_eq!(bare.readings_text(), "");
    assert!(split_of("", &table, false).is_none());
}

/// 繁简索引只收两列不同的行，其余字原样返回。
#[test]
fn simplified_map_falls_back_to_itself() {
    let dir = std::env::temp_dir().join("qingjian-chaizi-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("fanjian.tsv");
    std::fs::write(&path, "謝\t谢\n丁\t丁\n").unwrap();
    let table = Simplified::from_path(&path).unwrap();
    assert_eq!(table.simplify('謝'), '谢');
    assert_eq!(table.simplify('丁'), '丁');
    assert_eq!(table.simplify('木'), '木');
}
