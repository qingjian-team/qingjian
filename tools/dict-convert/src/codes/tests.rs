//! `pack codes` 的取码规则、笔画表解析与空输入边界。

use std::io::Cursor;
use std::path::{Path, PathBuf};

use qingjian_dictionary::AuxCodeTable;
use qingjian_format::Metadata;

use crate::codes::{CodeStats, StrokeTable, build, key, single_char_code, word_code};

/// 一次测试一个目录，结束删掉。
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("qingjian-codes-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    /// 写一个测试用的输入文件。
    fn write(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, contents).unwrap();
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// 解析一段笔画表文本。
fn table(source: &str) -> StrokeTable {
    StrokeTable::parse(Path::new("test"), Cursor::new(source)).unwrap()
}

/// 键位映射：1 h、2 s、3 p、5 z、点捺（4 与 n）n。
#[test]
fn maps_stroke_codes_to_keys() {
    assert_eq!(key('1'), Some('h'));
    assert_eq!(key('2'), Some('s'));
    assert_eq!(key('3'), Some('p'));
    assert_eq!(key('4'), Some('n'));
    assert_eq!(key('5'), Some('z'));
    assert_eq!(key('n'), Some('n'));
    assert_eq!(key('x'), None);
    assert_eq!(key(' '), None);

    let table = table("一\t1235\n丶\t4\n捺\tn\n# 注释\n");
    assert_eq!(table.get('一'), Some("hspz"));
    assert_eq!(table.get('丶'), Some("n"));
    assert_eq!(table.get('捺'), Some("n"));
    assert_eq!(table.get('二'), None);
    assert_eq!(table.len(), 3);
    assert!(!table.is_empty());
}

/// 笔画表的坏行直接报错，不静默吞掉。
#[test]
fn rejects_bad_stroke_table_lines() {
    for source in [
        "开\t1\nx\n",     // 缺第二个字段
        "开 1\n",         // 没有分隔符
        "开\tx\n",        // 认不出的笔画码（产物只有 1 2 3 5 与 n）
        "开\t\n",         // 空序列
        "开\t1\n开\t2\n", // 一个字两条序列
    ] {
        assert!(
            StrokeTable::parse(Path::new("test"), Cursor::new(source)).is_err(),
            "应当报错：{source}"
        );
    }
}

/// 单字：前 4 笔 + 末笔；不足 5 笔按实际取。
#[test]
fn single_char_takes_first_four_and_last() {
    assert_eq!(single_char_code("hspnz").as_deref(), Some("hspnz")); // 正好 5 笔
    assert_eq!(single_char_code("hspnzh").as_deref(), Some("hspnh")); // 6 笔：前 4 + 末
    assert_eq!(single_char_code("hspnhspn").as_deref(), Some("hspnn")); // 8 笔
    assert_eq!(single_char_code("hsp").as_deref(), Some("hsp")); // 3 画就是 3 码
    assert_eq!(single_char_code("h").as_deref(), Some("h")); // 1 画 1 码
    assert_eq!(single_char_code(""), None);
}

/// 词：单字走单字规则，二字 2 码、三字 3 码、四字及以上取前三字首笔 + 末字首笔。
#[test]
fn words_take_the_first_stroke_of_each_character() {
    let table = table("一\t1\n丨\t2\n丿\t3\n丶\t4\n乙\t5\n开\t123512\n");
    assert_eq!(word_code("一", &table).as_deref(), Some("h"));
    assert_eq!(word_code("开", &table).as_deref(), Some("hspzs")); // 单字 6 画
    assert_eq!(word_code("一丨", &table).as_deref(), Some("hs"));
    assert_eq!(word_code("一丨丿", &table).as_deref(), Some("hsp"));
    assert_eq!(word_code("一丨丿乙", &table).as_deref(), Some("hspz"));
    assert_eq!(word_code("一丨丿丶乙", &table).as_deref(), Some("hspz")); // 五字：第 4 字不取
    assert_eq!(word_code("", &table), None);
}

/// 词里有字不在笔画表里：整词没有码（按规则用不到的中间字也算）。
#[test]
fn words_with_an_unknown_character_have_no_code() {
    let table = table("一\t1\n丨\t2\n");
    assert_eq!(word_code("一丨", &table).as_deref(), Some("hs"));
    assert_eq!(word_code("一丁", &table), None);
    assert_eq!(word_code("一丨丿丶乙", &table), None);
    assert_eq!(word_code("abc", &table), None);
}

/// 空笔画表与空词库：产物是空码表，统计全 0，不报错。
#[test]
fn empty_inputs_produce_an_empty_code_table() {
    let dir = TempDir::new("empty");
    let stroke = dir.write("stroke.tsv", "");
    let dict = dir.write("dict.tsv", "");
    let out = dir.path().join("codes").join("stroke.qj");
    let metadata = Metadata {
        name: "笔画".to_owned(),
        ..Metadata::default()
    };
    let stats = build(&stroke, &dict, &out, &metadata).unwrap();
    assert_eq!(stats, CodeStats::default());

    let table = AuxCodeTable::open(&out).unwrap();
    assert!(table.is_empty());
    assert_eq!(table.len(), 0);
    assert_eq!(table.word_count(), 0);
    assert_eq!(table.metadata().unwrap().name, "笔画");
}

/// 走一遍完整管线：笔画表 + 词库 TSV → 码表 `.qj`，含缺字跳过与多音字词条去重。
#[test]
fn build_writes_codes_for_the_whole_dictionary() {
    let dir = TempDir::new("build");
    let stroke = dir.write(
        "stroke.tsv",
        "一\t1\n丨\t2\n丿\t3\n丶\t4\n乙\t5\n开\t123512\n",
    );
    let dict = dir.write(
        "dict.tsv",
        "一\tyi\t100\n一\tyi\t100\n一丨\tyi shu\t50\n一乙\tyi yi\t20\n开丁\tkai ding\t10\n",
    );
    let out = dir.path().join("codes").join("stroke.qj");
    let metadata = Metadata {
        name: "笔画".to_owned(),
        license: "OFL-1.1".to_owned(),
        ..Metadata::default()
    };

    let stats = build(&stroke, &dict, &out, &metadata).unwrap();
    assert_eq!(stats.entries, 5); // 词条数
    assert_eq!(stats.words, 4); // 去重后的词数（一 有两条词条）
    assert_eq!(stats.coded, 3);
    assert_eq!(stats.skipped, 1); // 开丁：丁 不在笔画表里

    let table = AuxCodeTable::open(&out).unwrap();
    assert_eq!(table.len(), 3);
    assert_eq!(table.word_count(), 3);
    assert_eq!(table.metadata().unwrap().entries, 3);
    assert_eq!(table.metadata().unwrap().license, "OFL-1.1");
    assert_eq!(table.codes_of("一").collect::<Vec<_>>(), ["h"]);
    assert_eq!(table.codes_of("一丨").collect::<Vec<_>>(), ["hs"]);
    assert_eq!(table.codes_of("一乙").collect::<Vec<_>>(), ["hz"]);
    assert_eq!(table.codes_of("开丁").count(), 0);
}
