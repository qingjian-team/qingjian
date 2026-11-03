//! 剪贴板里的代码：认出一段文字像不像代码（像就等宽显示、不重排），给它切出高亮片段（关键字、字符串、注释、数字）。
//! 不认语言，按多数语言共有的写法粗切：`//` `#` `--` 行注释、`/* */` 块注释、三种引号的字符串、数字、常见关键字的并集。
//! 只管显示，切错了也只是颜色不对；文字本身原样上屏。

mod highlight;
mod kind;
mod span;

pub use highlight::highlight;
pub use kind::CodeKind;
pub use span::CodeSpan;

/// 像不像代码：至少两行，且有缩进，或括号分号成堆。
pub fn looks_like_code(text: &str) -> bool {
    let mut lines = 0;
    let mut indented = false;
    for line in text.lines() {
        lines += 1;
        indented |= line.starts_with("  ") || line.starts_with('\t');
    }
    lines >= 2
        && (indented
            || text
                .chars()
                .filter(|c| matches!(c, '{' | '}' | ';'))
                .count()
                >= 3)
}

#[cfg(test)]
mod tests;
