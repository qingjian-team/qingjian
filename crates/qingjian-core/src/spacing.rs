//! 中西文混排自动空格（配置 `[general] auto_space`）：汉字与半角字母 / 数字相邻处补一个空格，
//! 「用Go写了30行」上屏成「用 Go 写了 30 行」。标点、全角字母与已有的空白两侧不补。

/// 汉字与假名。只认这些表意 / 音节文字，中文标点（，。「」）不算。
fn is_cjk(c: char) -> bool {
    matches!(
        u32::from(c),
        0x3040..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x3134F
    )
}

/// `prev` 后面紧接着 `next` 时中间要不要补空格。
pub fn needs_space(prev: char, next: char) -> bool {
    (is_cjk(prev) && next.is_ascii_alphanumeric()) || (prev.is_ascii_alphanumeric() && is_cjk(next))
}

/// 给一段文本内部的中西文交界补空格（整句候选「用docker部署」→「用 docker 部署」）。
pub fn space_inner(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 4);
    let mut prev = None;
    for c in text.chars() {
        if prev.is_some_and(|p| needs_space(p, c)) {
            out.push(' ');
        }
        out.push(c);
        prev = Some(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spaces_between_han_and_latin_or_digits() {
        assert!(needs_space('用', 'G'));
        assert!(needs_space('o', '写'));
        assert!(needs_space('余', '3'));
        assert!(needs_space('0', '天'));
        assert!(needs_space('の', 'A'));
    }

    #[test]
    fn leaves_punctuation_and_whitespace_alone() {
        assert!(!needs_space('，', 'G'));
        assert!(!needs_space('G', '。'));
        assert!(!needs_space(' ', '写'));
        assert!(!needs_space('用', ' '));
        assert!(!needs_space('3', '%'));
        assert!(!needs_space('用', 'Ｇ'));
        assert!(!needs_space('a', 'b'));
        assert!(!needs_space('你', '好'));
    }

    #[test]
    fn spaces_inside_mixed_text() {
        assert_eq!(space_inner("用docker部署"), "用 docker 部署");
        assert_eq!(space_inner("剩余30天"), "剩余 30 天");
        assert_eq!(space_inner("C盘"), "C 盘");
        assert_eq!(space_inner("用 Go 写"), "用 Go 写");
        assert_eq!(space_inner("你好，world"), "你好，world");
        assert_eq!(space_inner("hello"), "hello");
        assert_eq!(space_inner(""), "");
    }
}
