//! 剪贴板卡片的代码显示：像不像代码、高亮片段，偏移换成 UTF-16（Kotlin 的 Spannable 按它数）。

mod view;

pub use view::CodeView;

use qingjian_core::code::{highlight, looks_like_code};

/// 不像代码时 `code` 为假、没有片段（壳照普通文字显示）。
pub fn code_view(text: &str) -> CodeView {
    if !looks_like_code(text) {
        return CodeView {
            code: false,
            spans: Vec::new(),
        };
    }
    let utf16 = |byte: usize| text[..byte].encode_utf16().count();
    CodeView {
        code: true,
        spans: highlight(text)
            .into_iter()
            .map(|span| (utf16(span.range.start), utf16(span.range.end), span.kind))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_are_utf16() {
        let view = code_view("// 注释😀\nlet x = 1;\n  y;");
        assert!(view.code);
        assert_eq!((view.spans[0].0, view.spans[0].1), (0, 7));
        assert_eq!((view.spans[1].0, view.spans[1].1), (8, 11));
        assert!(!code_view("你好").code);
    }
}
