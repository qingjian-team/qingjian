//! 中文模式下的全角标点。
//!
//! 组句期间 `,` `.` 是翻页键、`-` `=` 也是，这些由壳决定；这里只回答「这个半角字符在中文模式下该变成什么」。

/// 这个半角字符是不是会转成全角标点的一员（`,` `.` `?` `!` 引号、括号这些）。
/// 壳用它分流组句中的标点：会转的先把高亮候选上屏再补标点（`nihao,` 一气打完「你好，」），
/// 其余（`-` `/` `@`）仍进英文直输段。数字后的 `.` 保持半角是 [`Punctuation::convert`] 的状态决定的，
/// 这里照算「会转」，上屏候选后交给它判断即可。
pub fn converts(c: char) -> bool {
    matches!(
        c,
        ',' | '.'
            | '?'
            | '!'
            | ':'
            | ';'
            | '('
            | ')'
            | '['
            | ']'
            | '<'
            | '>'
            | '\\'
            | '^'
            | '_'
            | '$'
            | '~'
            | '"'
            | '\''
    )
}

/// 引号成对切换的状态。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Punctuation {
    /// 下一个 `"` 是左引号。
    double_quote_open: bool,

    /// 下一个 `'` 是左引号。
    single_quote_open: bool,

    /// 上一个上屏字符是 ASCII 数字：`3.14` 里的 `.` 保持半角。
    after_digit: bool,
}

impl Punctuation {
    /// 半角字符对应的全角标点；不需要转换的返回 `None`，壳把原字符交给应用。
    pub fn convert(&mut self, c: char) -> Option<&'static str> {
        let after_digit = std::mem::replace(&mut self.after_digit, false);
        let converted = match c {
            ',' => "，",
            '.' if after_digit => return None,
            '.' => "。",
            '?' => "？",
            '!' => "！",
            ':' => "：",
            ';' => "；",
            '(' => "（",
            ')' => "）",
            '[' => "【",
            ']' => "】",
            '<' => "《",
            '>' => "》",
            '\\' => "、",
            '^' => "……",
            '_' => "——",
            '$' => "￥",
            '~' => "～",
            '"' => {
                self.double_quote_open = !self.double_quote_open;
                if self.double_quote_open { "“" } else { "”" }
            }
            '\'' => {
                self.single_quote_open = !self.single_quote_open;
                if self.single_quote_open { "‘" } else { "’" }
            }
            _ => return None,
        };
        Some(converted)
    }

    /// 壳把没有转换的字符原样交给应用后调用，用来记住「刚打了数字」。
    pub fn note_passthrough(&mut self, c: char) {
        self.after_digit = c.is_ascii_digit();
    }

    /// 有文本上屏后调用（候选、拼音、英文词），数字状态按最后一个字符更新。
    pub fn note_committed(&mut self, text: &str) {
        self.after_digit = text.chars().last().is_some_and(|c| c.is_ascii_digit());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_common_marks_and_toggles_quotes() {
        let mut p = Punctuation::default();
        assert_eq!(p.convert(','), Some("，"));
        assert_eq!(p.convert('"'), Some("“"));
        assert_eq!(p.convert('"'), Some("”"));
        assert_eq!(p.convert('\''), Some("‘"));
        assert_eq!(p.convert('\''), Some("’"));
        assert_eq!(p.convert('a'), None);
        assert_eq!(p.convert('-'), None);
    }

    #[test]
    fn period_after_digit_stays_ascii() {
        let mut p = Punctuation::default();
        p.note_passthrough('3');
        assert_eq!(p.convert('.'), None);
        assert_eq!(p.convert('.'), Some("。"));
        p.note_committed("第1");
        assert_eq!(p.convert('.'), None);
        p.note_committed("开发");
        assert_eq!(p.convert('.'), Some("。"));
    }

    /// `converts` 与 `convert` 能转的字符集合一致（`after_digit` 的 `.` 除外，那是状态决定的）。
    #[test]
    fn converts_covers_exactly_the_convertible_characters() {
        for c in ('\t'..='~').filter(char::is_ascii_graphic) {
            let convertible = Punctuation::default().convert(c).is_some();
            assert_eq!(converts(c), convertible, "{c}");
        }
        assert!(converts('\''));
        assert!(!converts('a'));
        assert!(!converts('-'));
        assert!(!converts('/'));
        assert!(!converts('@'));
        assert!(!converts('中'));
    }
}
