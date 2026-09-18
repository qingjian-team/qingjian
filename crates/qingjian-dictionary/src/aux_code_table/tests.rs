//! 码表的解析、落盘与查询。

use std::path::{Path, PathBuf};

use qingjian_format::{Container, Kind, Metadata};

use super::parsed::ParsedTable;
use super::{AuxCodeLookup, AuxCodeTable, aux_code_table_info, import_aux_code_table};
use crate::error::DictionaryError;

/// 一次测试一个目录，结束删掉。
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("qingjian-code-table-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn sample() -> AuxCodeTable {
    AuxCodeTable::from_pairs([
        ("开发".to_owned(), "kf".to_owned()),
        ("开发".to_owned(), "kaifa".to_owned()),
        ("开放".to_owned(), "kf".to_owned()),
        ("鹤".to_owned(), "hn".to_owned()),
    ])
    .unwrap()
}

#[test]
fn pairs_are_sorted_and_deduplicated() {
    let table = AuxCodeTable::from_pairs([
        ("开放".to_owned(), "kf".to_owned()),
        ("开发".to_owned(), "kf".to_owned()),
        ("开发".to_owned(), "kf".to_owned()),
        ("开发".to_owned(), "kaifa".to_owned()),
    ])
    .unwrap();
    assert_eq!(table.len(), 3);
    assert_eq!(table.word_count(), 2);
    let listed: Vec<(&str, &str)> = table.entries().collect();
    assert_eq!(listed, [("开发", "kaifa"), ("开发", "kf"), ("开放", "kf")]);
}

#[test]
fn codes_of_and_prefix_lookup_see_every_code() {
    let table = sample();
    let codes: Vec<&str> = table.codes_of("开发").collect();
    assert_eq!(codes, ["kaifa", "kf"]);
    assert_eq!(table.code_with_prefix("开发", "k"), Some("kaifa"));
    assert_eq!(table.code_with_prefix("开发", "kf"), Some("kf"));
    assert_eq!(table.code_with_prefix("鹤", "h"), Some("hn"));
    assert_eq!(table.code_with_prefix("鹤", "k"), None);
    assert_eq!(table.code_with_prefix("没有这个词", "k"), None);
    assert_eq!(table.codes_of("开放").count(), 1);
}

#[test]
fn rejects_codes_outside_the_alphabet() {
    assert!(AuxCodeTable::from_pairs([("鹤".to_owned(), "H1".to_owned())]).is_err());
    assert!(AuxCodeTable::from_pairs([("鹤".to_owned(), "h".repeat(9))]).is_err());
    assert!(AuxCodeTable::from_pairs([("鹤".to_owned(), String::new())]).is_err());
}

#[test]
fn qj_round_trip_keeps_every_lookup() {
    let dir = TempDir::new("round-trip");
    let path = dir.path().join("stroke.qj");
    let table = sample();
    let metadata = Metadata {
        name: "笔画".to_owned(),
        license: "OFL-1.1".to_owned(),
        attribution: "CNS11643".to_owned(),
        ..Metadata::default()
    };
    table.write_qj(&path, &metadata).unwrap();
    let mapped = AuxCodeTable::open(&path).unwrap();
    assert_eq!(mapped.metadata().unwrap().name, "笔画");
    assert_eq!(mapped.metadata().unwrap().entries, 4);
    assert_eq!(mapped.len(), table.len());
    assert_eq!(mapped.word_count(), table.word_count());
    for word in ["开发", "开放", "鹤", "没有这个词"] {
        let before: Vec<&str> = table.codes_of(word).collect();
        let after: Vec<&str> = mapped.codes_of(word).collect();
        assert_eq!(before, after, "{word}");
    }
    assert_eq!(mapped.code_with_prefix("开发", "kf"), Some("kf"));
}

#[test]
fn another_kind_is_not_an_aux_code_table() {
    let dir = TempDir::new("wrong-kind");
    let path = dir.path().join("dict.qj");
    qingjian_format::Writer::new(Kind::Dictionary, &Metadata::default())
        .unwrap()
        .section(*b"TEXT", b"x")
        .write_to(&path)
        .unwrap();
    assert!(matches!(
        AuxCodeTable::open(&path),
        Err(crate::DictionaryError::Format(
            qingjian_format::FormatError::WrongKind { .. }
        ))
    ));
    assert!(Container::is_qj(&path));
}

#[test]
fn parses_columns_and_counts_missing_codes() {
    let text = "---\nname: 形码\ncolumns: [text, weight]\n...\n开发\t100\n开放\t50\n";
    let parsed = ParsedTable::parse(text);
    assert_eq!(parsed.columns.text, 0);
    assert_eq!(parsed.columns.code, None);
    assert_eq!(parsed.read, 2);
    assert_eq!(parsed.no_code, 2);
    assert_eq!(parsed.with_code, 0);
    assert_eq!(parsed.skipped, 0);
    assert!(parsed.pairs.is_empty());
}

#[test]
fn inline_and_block_lists_both_parse() {
    let parsed = ParsedTable::parse(
        "---\nname: 形码\ncolumns: [text, code, weight]\nimport_tables: [base]\n...\n开发\tkf\n",
    );
    assert_eq!(parsed.columns.code, Some(1));
    assert_eq!(parsed.import_tables, ["base"]);
    assert_eq!(parsed.pairs, [("开发".to_owned(), "kf".to_owned())]);
}

/// 无 YAML 头的纯 TSV：首行 `#` 注释与空行不再把整份正文当头吞掉（曾经整份导入失败报 `NoCodeEntries`），
/// 统计照常。
#[test]
fn headerless_tsv_tolerates_leading_comments_and_blank_lines() {
    let text = "# 一份社区码表\n\n开发\tkf\n开放\tkf\n";
    let (_dir, imported) = parse_via_import("headerless", text);
    assert_eq!(imported.report.read, 2);
    assert_eq!(imported.report.with_code, 2);
    assert_eq!(imported.report.no_code, 0);
    let table = AuxCodeTable::open(&imported.path).unwrap();
    assert_eq!(table.len(), 2);
    assert_eq!(table.codes_of("开发").count(), 1);
}

/// 走一遍导入：写文件 → `import_aux_code_table` → 读回来。目录跟着结果一起返回，调用方拿着它别删。
fn parse_via_import(name: &str, text: &str) -> (TempDir, super::AuxCodeTableImport) {
    let dir = TempDir::new(name);
    let source = dir.path().join(format!("{name}.dict.yaml"));
    std::fs::write(&source, text).unwrap();
    let imported = import_aux_code_table(&source, &dir.path().join("codes")).unwrap();
    (dir, imported)
}

#[test]
fn default_columns_read_word_and_code() {
    let text = "---\nname: 形码\n...\n开发\tkf\t100\n开放\tkf\n开发\tkaifa\n";
    let (_dir, imported) = parse_via_import("default", text);
    assert_eq!(imported.name, "形码");
    assert_eq!(imported.report.read, 3);
    assert_eq!(imported.report.with_code, 3);
    assert_eq!(imported.report.no_code, 0);
    let table = AuxCodeTable::open(&imported.path).unwrap();
    assert_eq!(table.len(), 3);
    assert_eq!(table.codes_of("开发").count(), 2);
}

#[test]
fn import_tables_are_merged_relative_to_the_main_file() {
    let dir = TempDir::new("import-tables");
    std::fs::create_dir_all(dir.path().join("sub")).unwrap();
    std::fs::write(
        dir.path().join("sub/stem.dict.yaml"),
        "---\nname: 字根\n...\n鹤\thn\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("main.dict.yaml"),
        "---\nname: 主表\nimport_tables:\n  - sub/stem\n...\n开发\tkf\n",
    )
    .unwrap();
    let imported = import_aux_code_table(
        &dir.path().join("main.dict.yaml"),
        &dir.path().join("codes"),
    )
    .unwrap();
    assert_eq!(imported.name, "主表");
    assert_eq!(imported.report.read, 2);
    let table = AuxCodeTable::open(&imported.path).unwrap();
    assert_eq!(table.len(), 2);
    assert_eq!(table.code_with_prefix("开发", "k"), Some("kf"));
    assert_eq!(table.code_with_prefix("鹤", "h"), Some("hn"));
}

#[test]
fn skips_bad_codes_but_keeps_the_rest() {
    let text = "---\nname: 杂\n...\n开发\tkf\n开放\tK1\n鹤\t\n开\thn zz\n";
    let (_dir, imported) = parse_via_import("bad", text);
    assert_eq!(imported.report.read, 4);
    assert_eq!(imported.report.with_code, 2);
    assert_eq!(imported.report.no_code, 1);
    assert_eq!(imported.report.skipped, 1);
    let table = AuxCodeTable::open(&imported.path).unwrap();
    // `hn zz` 里空格分开的两个码：合法的收下，非法的丢掉
    assert_eq!(table.code_with_prefix("开", "h"), Some("hn"));
}

#[test]
fn a_file_without_any_code_is_rejected() {
    let dir = TempDir::new("no-code");
    let source = dir.path().join("words.dict.yaml");
    std::fs::write(
        &source,
        "---\nname: 纯词表\ncolumns: [text, weight]\n...\n开发\t100\n",
    )
    .unwrap();
    assert!(matches!(
        import_aux_code_table(&source, &dir.path().join("codes")),
        Err(crate::DictionaryError::NoCodeEntries)
    ));
}

#[test]
fn info_reports_broken_files_without_failing() {
    let dir = TempDir::new("info");
    let good = dir.path().join("stroke.qj");
    sample()
        .write_qj(
            &good,
            &Metadata {
                name: "笔画".to_owned(),
                license: "OFL-1.1".to_owned(),
                ..Metadata::default()
            },
        )
        .unwrap();
    let info = aux_code_table_info(&good);
    assert_eq!(info.name, "笔画");
    assert_eq!(info.entries, 4);
    assert_eq!(info.license, "OFL-1.1");
    assert!(!info.broken);
    let broken = dir.path().join("broken.qj");
    std::fs::write(&broken, b"not a qj file at all, and long enough").unwrap();
    let info = aux_code_table_info(&broken);
    assert!(info.broken);
    assert_eq!(info.name, "broken");
    assert_eq!(info.entries, 0);
}

#[test]
fn tsv_parse_reads_word_and_code() {
    let table = AuxCodeTable::parse("# 注释\n开发\tkf\n\n鹤\thn\n").unwrap();
    assert_eq!(table.len(), 2);
    assert!(AuxCodeTable::parse("开发\tkf\n鹤\tH1\n").is_err());
}

/// 同一入口吃两种文本：Rime `.dict.yaml`（与导入同一套解析）与 `词\t码` TSV。
#[test]
fn reads_rime_dict_yaml_and_tsv_from_the_same_entry_point() {
    let rime = "---\nname: 形码\ncolumns: [text, code, weight]\n...\n开发\tkf\t100\n开放\tkfang\n";
    let table = AuxCodeTable::from_text(rime).unwrap();
    assert_eq!(table.len(), 2);
    assert_eq!(table.code_with_prefix("开发", "k"), Some("kf"));

    let tsv = "# 注释\n开发\tkf\n";
    assert_eq!(AuxCodeTable::from_text(tsv).unwrap().len(), 1);

    // 纯词表（没有码列）报错，不静默出一张空表
    assert!(matches!(
        AuxCodeTable::from_text("---\nname: 纯词表\ncolumns: [text, weight]\n...\n开发\t100\n"),
        Err(DictionaryError::NoCodeEntries)
    ));
}
