//! 粗切高亮片段：一遍扫过去，注释、字符串先认（里面的关键字不算），再认数字与关键字。

use super::{CodeKind, CodeSpan};

/// 常见语言关键字的并集（Rust、C 系、Java / Kotlin、Python、JS / TS、Go、Swift、Shell、SQL 小写）。
const KEYWORDS: &[&str] = &[
    "abstract",
    "and",
    "as",
    "async",
    "await",
    "bool",
    "boolean",
    "break",
    "case",
    "catch",
    "char",
    "class",
    "const",
    "continue",
    "def",
    "default",
    "defer",
    "del",
    "do",
    "double",
    "elif",
    "else",
    "end",
    "enum",
    "except",
    "export",
    "extends",
    "false",
    "final",
    "finally",
    "float",
    "fn",
    "for",
    "from",
    "fun",
    "func",
    "function",
    "go",
    "if",
    "impl",
    "implements",
    "import",
    "in",
    "int",
    "interface",
    "is",
    "lambda",
    "let",
    "long",
    "loop",
    "match",
    "mod",
    "mut",
    "new",
    "nil",
    "none",
    "not",
    "null",
    "object",
    "or",
    "override",
    "package",
    "pass",
    "private",
    "protected",
    "pub",
    "public",
    "raise",
    "return",
    "select",
    "self",
    "static",
    "string",
    "struct",
    "super",
    "switch",
    "then",
    "this",
    "throw",
    "throws",
    "trait",
    "true",
    "try",
    "type",
    "typeof",
    "use",
    "val",
    "var",
    "void",
    "when",
    "where",
    "while",
    "with",
    "yield",
    "None",
    "True",
    "False",
    "Self",
    "String",
    "Some",
    "Ok",
    "Err",
];

/// 切出高亮片段，按出现顺序、不重叠。
pub fn highlight(text: &str) -> Vec<CodeSpan> {
    let bytes = text.as_bytes();
    let mut spans = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let rest = &text[i..];
        let start = i;
        let line_start = i == 0 || bytes[i - 1] == b'\n' || bytes[i - 1].is_ascii_whitespace();
        if rest.starts_with("//")
            || (rest.starts_with('#') && line_start)
            || rest.starts_with("-- ")
        {
            i += rest.find('\n').unwrap_or(rest.len());
            spans.push(span(start, i, CodeKind::Comment));
        } else if let Some(body) = rest.strip_prefix("/*") {
            i += body.find("*/").map_or(rest.len(), |end| end + 4);
            spans.push(span(start, i, CodeKind::Comment));
        } else if let Some(quote @ (b'"' | b'\'' | b'`')) = bytes.get(i).copied() {
            match string_end(bytes, i, quote) {
                Some(end) => {
                    spans.push(span(start, end, CodeKind::String));
                    i = end;
                }
                // 找不到收尾的引号（Rust 的生命周期 'a、英文缩写）：只当一个字符
                None => i += 1,
            }
        } else if bytes[i].is_ascii_digit() && !after_word(bytes, i) {
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'.' | b'_'))
            {
                i += 1;
            }
            spans.push(span(start, i, CodeKind::Number));
        } else if is_word_start(bytes[i]) {
            while i < bytes.len() && is_word(bytes[i]) {
                i += 1;
            }
            if KEYWORDS.contains(&&text[start..i]) {
                spans.push(span(start, i, CodeKind::Keyword));
            }
        } else {
            // 跳过一个完整字符（中文注释外的汉字也走这里）
            i += rest.chars().next().map_or(1, char::len_utf8);
        }
    }
    spans
}

fn span(start: usize, end: usize, kind: CodeKind) -> CodeSpan {
    CodeSpan {
        range: start..end,
        kind,
    }
}

/// 从 `start` 的引号起找收尾的同一种引号（跳过反斜杠转义）；单双引号不跨行，反引号可以。返回收尾引号之后的位置。
fn string_end(bytes: &[u8], start: usize, quote: u8) -> Option<usize> {
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'\n' if quote != b'`' => return None,
            b if b == quote => return Some(i + 1),
            _ => i += 1,
        }
    }
    None
}

fn is_word_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// 数字紧跟在标识符后面（`x2`、`utf8`）就不是数字。
fn after_word(bytes: &[u8], i: usize) -> bool {
    i > 0 && is_word(bytes[i - 1])
}
