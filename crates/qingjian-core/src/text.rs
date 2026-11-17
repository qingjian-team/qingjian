//! 按字符类别找文字边界的几把尺子。
//!
//! 壳里的「删一个词」「横向滑一格挪一个字」这类手势要用它，各平台共用一个实现，
//! 免得到安卓、iOS、鸿蒙上各写一份、边界规则还各不相同。

/// 光标前那个「词」有多长（`before` 末尾往回数），删一词手势按它定边界。
///
/// 拉丁字母、数字、下划线、连字符、撇号连成一串算一个词（`hello_world`、`version-2`、`it's`
/// 都是整串），汉字、假名、标点一个字符算一个；`before` 以空白结尾时先吃掉那一段连续的空白。
///
/// 只看字符类别、不看词库：正文已经上屏，结果不该因为学了新词而变。
///
/// ```
/// use qingjian_core::text::word_tail_length;
///
/// assert_eq!(word_tail_length("hello_world"), 11);
/// assert_eq!(word_tail_length("hello world"), 5);
/// assert_eq!(word_tail_length("你好世界"), 1);
/// assert_eq!(word_tail_length("abc   "), 3);
/// assert_eq!(word_tail_length(""), 0);
/// ```
pub fn word_tail_length(before: &str) -> usize {
    let Some(last) = before.chars().next_back() else {
        return 0;
    };
    if last.is_whitespace() {
        return before.chars().rev().take_while(|c| c.is_whitespace()).count();
    }
    if !is_word_char(last) {
        return 1;
    }
    before.chars().rev().take_while(|c| is_word_char(*c)).count()
}

/// 算不算「一个词里的字符」：拉丁字母、数字、下划线、连字符与撇号。壳里的删除手势据此判断\n/// 按下删除键那一下是不是已经删掉了一整个「一段」。
///
/// 汉字、假名、全角标点、空白都不算 —— 那些一个字符就是一段。
pub fn is_word_char(c: char) -> bool {
    if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '\'' {
        return true;
    }
    // 带音标的拉丁字母（café、naïve）在拉丁扩展与拉丁扩展补充里；中间夹着的 × ÷ 不是字母
    (0xC0..=0x24F).contains(&(c as u32)) && c != '×' && c != '÷'
}

#[cfg(test)]
mod tests {
    use super::word_tail_length;

    /// 一行一个用例：按下时的正文、该删掉几个字符、说明。
    const CASES: &[(&str, usize, &str)] = &[
        ("hello_world", 11, "拉丁与下划线连成一串"),
        ("hello world", 5, "空格断开"),
        ("hello wor", 3, "没打完的词也是整串"),
        ("你好世界", 1, "汉字一个字符算一个"),
        ("你好，世界", 1, "标点不断开汉字"),
        ("abc   ", 3, "结尾空白整段吃掉"),
        ("café", 4, "带音标的拉丁字母算在词里"),
        ("version-2", 9, "连字符算在词里"),
        ("it's", 4, "撇号算在词里"),
        ("3.14", 2, "点不断开数字，只删最后一段"),
        ("a", 1, "只剩一个字"),
        (" ", 1, "只有空白"),
        ("", 0, "空串"),
    ];

    #[test]
    fn the_tail_is_measured_by_character_class() {
        for (before, want, note) in CASES {
            assert_eq!(word_tail_length(before), *want, "{before:?}：{note}");
        }
    }

    #[test]
    fn a_whole_word_covers_the_whole_string() {
        assert_eq!(word_tail_length("hello_world"), "hello_world".len());
    }
}
