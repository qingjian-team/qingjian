//! 认代码与切高亮片段。

use super::*;

fn kinds(text: &str) -> Vec<(&str, CodeKind)> {
    highlight(text)
        .into_iter()
        .map(|span| (&text[span.range], span.kind))
        .collect()
}

#[test]
fn recognises_code_shapes() {
    assert!(looks_like_code("fn main() {\n    run();\n}"));
    assert!(looks_like_code("a = 1;\nb = 2;\nc = 3;"));
    assert!(!looks_like_code("你好，明天见"));
    assert!(!looks_like_code("第一行\n第二行"));
}

#[test]
fn keywords_strings_numbers_and_comments() {
    let text = "let x = \"for\"; // 注释 return\nif y > 42 { return 'c'; }";
    assert_eq!(
        kinds(text),
        [
            ("let", CodeKind::Keyword),
            ("\"for\"", CodeKind::String),
            ("// 注释 return", CodeKind::Comment),
            ("if", CodeKind::Keyword),
            ("42", CodeKind::Number),
            ("return", CodeKind::Keyword),
            ("'c'", CodeKind::String),
        ]
    );
}

#[test]
fn block_comments_hash_comments_and_escapes() {
    let text = "/* a\n b */ x = \"a\\\"b\"\n# def";
    assert_eq!(
        kinds(text),
        [
            ("/* a\n b */", CodeKind::Comment),
            ("\"a\\\"b\"", CodeKind::String),
            ("# def", CodeKind::Comment),
        ]
    );
}

#[test]
fn lone_quotes_and_numbers_in_names_are_plain() {
    assert_eq!(kinds("fn f<'a>(utf8: x2)"), [("fn", CodeKind::Keyword)]);
}
