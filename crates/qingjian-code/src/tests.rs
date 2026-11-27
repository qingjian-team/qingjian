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

#[test]
fn dash_line_comments_and_unclosed_strings() {
    assert_eq!(
        kinds("-- 说明\nlet x = \"未闭合"),
        [("-- 说明", CodeKind::Comment), ("let", CodeKind::Keyword)]
    );
    assert_eq!(kinds("let a = 'x\ny"), [("let", CodeKind::Keyword)]);
    assert_eq!(kinds("`跨\n行`"), [("`跨\n行`", CodeKind::String)]);
}

#[test]
fn escaped_newline_does_not_extend_a_string_across_lines() {
    assert_eq!(kinds("let a = \"x\\\ny\""), [("let", CodeKind::Keyword)]);
}

#[test]
fn empty_text_has_no_spans() {
    assert!(highlight("").is_empty());
    assert!(highlight("   \n\t").is_empty());
}

#[test]
fn code_shapes_need_two_lines_and_a_hint() {
    assert!(!looks_like_code(""));
    assert!(!looks_like_code("a; b; c;"));
    assert!(!looks_like_code("第一行\n第二行"));
    assert!(!looks_like_code("a b\nc d"));
    assert!(looks_like_code("a\n\tb"));
    assert!(looks_like_code("a\n  b"));
    assert!(looks_like_code("x = 1;\ny = 2;\nz = 3;"));
    assert!(!looks_like_code("x = 1;\ny = 2"));
}

#[test]
fn a_line_comment_runs_to_the_end_of_the_line() {
    assert_eq!(kinds("// 只到行尾"), [("// 只到行尾", CodeKind::Comment)]);
    assert_eq!(
        kinds("let a = 1; // x\nlet b = 2;"),
        [
            ("let", CodeKind::Keyword),
            ("1", CodeKind::Number),
            ("// x", CodeKind::Comment),
            ("let", CodeKind::Keyword),
            ("2", CodeKind::Number),
        ]
    );
}

#[test]
fn hash_comments_need_a_line_start() {
    assert_eq!(kinds("# 说明"), [("# 说明", CodeKind::Comment)]);
    assert_eq!(
        kinds("a = 1 # 说明"),
        [("1", CodeKind::Number), ("# 说明", CodeKind::Comment),]
    );
    // 紧跟在标识符后面的 # 不是注释，也不是关键字，整段没有可上色的
    assert!(kinds("a#b").is_empty());
}

#[test]
fn dash_comments_need_a_trailing_space() {
    assert_eq!(kinds("-- 注释"), [("-- 注释", CodeKind::Comment)]);
    assert_eq!(kinds("a -- b"), [("-- b", CodeKind::Comment)]);
    assert!(kinds("a--b").is_empty());
}

#[test]
fn unclosed_block_comment_runs_to_the_end() {
    assert_eq!(kinds("/* 没有收尾"), [("/* 没有收尾", CodeKind::Comment)]);
    assert_eq!(
        kinds("/* a */ let x = /* b */ 1;"),
        [
            ("/* a */", CodeKind::Comment),
            ("let", CodeKind::Keyword),
            ("/* b */", CodeKind::Comment),
            ("1", CodeKind::Number),
        ]
    );
}

#[test]
fn strings_hide_keywords_and_comments() {
    assert_eq!(
        kinds("let a = \"fn // not a comment\";"),
        [
            ("let", CodeKind::Keyword),
            ("\"fn // not a comment\"", CodeKind::String),
        ]
    );
}

#[test]
fn escaped_quotes_stay_inside_the_string() {
    assert_eq!(
        kinds(r#"let s = "a\"b";"#),
        [("let", CodeKind::Keyword), (r#""a\"b""#, CodeKind::String)]
    );
}

#[test]
fn unterminated_quotes_are_single_characters() {
    assert_eq!(kinds("let a = 'x"), [("let", CodeKind::Keyword)]);
    assert!(kinds("it's ok").is_empty());
}

#[test]
fn backticks_may_span_lines() {
    assert_eq!(kinds("`a\nb` x"), [("`a\nb`", CodeKind::String)]);
}

#[test]
fn numbers_absorb_letters_dots_and_underscores() {
    assert_eq!(kinds("x = 1.5;"), [("1.5", CodeKind::Number)]);
    assert_eq!(kinds("y = 0x1F;"), [("0x1F", CodeKind::Number)]);
    assert_eq!(kinds("z = 1e5;"), [("1e5", CodeKind::Number)]);
    // 数字紧跟在标识符后面就不算数字；点开头的只从数字本身切
    assert!(kinds("x2").is_empty());
    assert_eq!(kinds(".5"), [("5", CodeKind::Number)]);
}

#[test]
fn keywords_are_case_sensitive() {
    assert!(kinds("Fn main()").is_empty());
    assert_eq!(kinds("None"), [("None", CodeKind::Keyword)]);
    assert_eq!(kinds("true"), [("true", CodeKind::Keyword)]);
}

#[test]
fn longer_identifiers_are_not_keywords() {
    assert!(kinds("fnx").is_empty());
    assert!(kinds("selfie").is_empty());
    assert_eq!(kinds("fn"), [("fn", CodeKind::Keyword)]);
}

#[test]
fn multibyte_characters_keep_byte_ranges() {
    let text = "中文 let 值";
    let spans = highlight(text);
    assert_eq!(spans.len(), 1);
    assert_eq!(&text[spans[0].range.clone()], "let");
}

#[test]
fn spans_are_ordered_and_do_not_overlap() {
    let text = "// a\nlet s = \"x\";\nif n > 1 { /* b */ }";
    let spans = highlight(text);
    let mut end = 0;
    for span in &spans {
        assert!(span.range.start >= end, "{span:?}");
        assert!(span.range.end > span.range.start, "{span:?}");
        end = span.range.end;
    }
}

#[test]
fn keywords_inside_comments_are_not_coloured() {
    assert_eq!(
        kinds("// let fn return"),
        [("// let fn return", CodeKind::Comment)]
    );
}

#[test]
fn a_string_ends_at_the_matching_quote_only() {
    assert_eq!(kinds(r#""a'b""#), [(r#""a'b""#, CodeKind::String)]);
}

#[test]
fn numbers_at_the_start_of_a_string_are_plain_text() {
    assert_eq!(
        kinds("\"42\" 42"),
        [("\"42\"", CodeKind::String), ("42", CodeKind::Number)]
    );
}

#[test]
fn a_comment_at_the_start_of_the_file_is_a_comment() {
    assert_eq!(
        kinds("# header\nlet x = 1;"),
        [
            ("# header", CodeKind::Comment),
            ("let", CodeKind::Keyword),
            ("1", CodeKind::Number),
        ]
    );
}
