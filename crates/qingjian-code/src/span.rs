//! 一段要上色的文字。

use std::ops::Range;

use super::CodeKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeSpan {
    /// 在原文里的字节范围（壳要 UTF-16 偏移的自己换算）。
    pub range: Range<usize>,

    pub kind: CodeKind,
}
