//! 单文件转换兼容性与自动注音组合上限。

use super::{parse, readings, to_tsv};

#[test]
fn single_file_conversion_preserves_shape_codes_and_weight_fallback() {
    let parsed = to_tsv("# Rime dictionary\n---\nname: wubi\n...\n青简\tggll\t1e5\n");
    assert_eq!(parsed.name.as_deref(), Some("wubi"));
    assert_eq!(parsed.tsv, "青简\tggll\t1\n");
}

#[test]
fn weight_column_is_never_treated_as_pinyin() {
    let parsed = to_tsv("---\nname: words\ncolumns: [text, weight]\n...\n青简\t100\n");
    assert!(parsed.tsv.is_empty());
}

#[test]
fn zero_weight_readings_and_combination_limit() {
    let (_, entries) = parse("---\nname: chars\n...\n长\tchang\t0\n长\tzhang\t0\n");
    let readings = readings::collect(&entries);
    assert_eq!(readings::encode("长", &readings).unwrap().len(), 2);
    assert_eq!(
        readings::encode(&"长".repeat(8), &readings).unwrap().len(),
        256
    );
    assert!(readings::encode(&"长".repeat(9), &readings).is_none());
    assert!(readings::encode("缺", &readings).is_none());
}

#[test]
fn headerless_shape_table_and_quoted_hash_in_name() {
    assert_eq!(to_tsv("# 注释\n\n青简\tggll\t2\n").tsv, "青简\tggll\t2\n");
    let parsed = to_tsv("---\nname: '词库 #1' # 注释\n...\n青简\tqing jian\n");
    assert_eq!(parsed.name.as_deref(), Some("词库 #1"));
}
