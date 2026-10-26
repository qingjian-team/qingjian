//! 把一段任意文本切成可以点选的小块（剪贴板的「拆词」）。块按原文顺序，**全部拼起来正好是原文**，
//! 所以挑几块拼起来就是那几块的原样（代码的换行缩进都在）：
//! - 汉字按语言模型切词（[`segment_text`]）；
//! - 空白也是块：一串空格 / 制表符算一块，换行单独一块（壳画成 ␣ / ↵，换行处另起一行）；
//! - 不带空白的 ASCII 串里，链接、邮箱、路径（有 `/ . @ :`）去掉首尾括号引号后整块；
//!   数字之间夹 `- . , : / +` 的算一个数（电话、日期、时间、千分位、小数），`+86` 开头的号码也整块；
//!   其余英文单词一块，标点一个一块，连着的同一个运算符（`//` `==` `&&`）合成一块。

use super::{LanguageModel, is_han, segment_text};

/// 一串不带空白的 ASCII 里，链接、邮箱、路径常见的连接符；串里有这些就整块不拆。
const LINK_MARKS: [char; 4] = ['/', '.', '@', ':'];

/// 夹在两段数字之间仍算同一个数的符号。
const NUMBER_JOINERS: [char; 6] = ['-', '.', ',', ':', '/', '+'];

/// 连着出现时合成一块的运算符（括号、引号不合，`((` 还是两块）。
const REPEATABLE: &str = "/=-+*&|<>:.!?#%~";

/// 切成小块。
pub fn text_pieces(text: &str, model: &dyn LanguageModel) -> Vec<String> {
    let mut pieces = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let end = if c == '\r' && chars.get(i + 1) == Some(&'\n') {
            i + 2
        } else if c == '\n' || c == '\r' {
            i + 1
        } else if c.is_whitespace() {
            run_end(&chars, i, |c| c.is_whitespace() && c != '\n' && c != '\r')
        } else if is_han(c) {
            let end = run_end(&chars, i, is_han);
            let run: String = chars[i..end].iter().collect();
            match segment_text(&run, model) {
                Some(clauses) => pieces.extend(clauses.into_iter().flatten()),
                // 模型一个词都不认识（或没有模型）：一个字一块
                None => pieces.extend(run.chars().map(String::from)),
            }
            i = end;
            continue;
        } else if c.is_ascii_graphic() {
            let end = run_end(&chars, i, |c| c.is_ascii_graphic());
            ascii_pieces(&chars[i..end], &mut pieces);
            i = end;
            continue;
        } else {
            i + 1
        };
        pieces.push(chars[i..end].iter().collect());
        i = end;
    }
    pieces
}

fn run_end(chars: &[char], start: usize, keep: impl Fn(char) -> bool) -> usize {
    chars[start..]
        .iter()
        .position(|&c| !keep(c))
        .map_or(chars.len(), |offset| start + offset)
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '\''
}

/// 一串不带空白的 ASCII：像链接就去掉首尾的括号引号后整块，否则拆成数、单词与标点。
fn ascii_pieces(run: &[char], pieces: &mut Vec<String>) {
    let body_start = run.iter().position(|&c| is_word(c)).unwrap_or(run.len());
    let body_end = run
        .iter()
        .rposition(|&c| is_word(c))
        .map_or(body_start, |i| i + 1);
    let body = &run[body_start..body_end];
    if body.iter().any(|c| LINK_MARKS.contains(c)) && body.iter().any(char::is_ascii_alphabetic) {
        pieces.extend(run[..body_start].iter().map(char::to_string));
        pieces.push(body.iter().collect());
        pieces.extend(run[body_end..].iter().map(char::to_string));
        return;
    }
    let mut i = 0;
    while i < run.len() {
        let end = if let Some(end) = number_end(run, i) {
            end
        } else if is_word(run[i]) {
            run_end(run, i, is_word)
        } else if REPEATABLE.contains(run[i]) {
            run_end(run, i, |c| c == run[i])
        } else {
            i + 1
        };
        pieces.push(run[i..end].iter().collect());
        i = end;
    }
}

/// 从 `start` 起是一个数（可带开头的 `+`）就返回它的结尾：数字段之间夹单个 [`NUMBER_JOINERS`] 仍算一个数，
/// 数后面紧跟字母（`3rd`、`utf8` 里的 8 不会走到这里）就整个当单词。不是数返回 `None`。
fn number_end(run: &[char], start: usize) -> Option<usize> {
    let mut i = start;
    if run[i] == '+' {
        i += 1;
    }
    if !run.get(i)?.is_ascii_digit() {
        return None;
    }
    loop {
        i = run_end(run, i, |c| c.is_ascii_digit());
        let joined = run.get(i).is_some_and(|c| NUMBER_JOINERS.contains(c))
            && run.get(i + 1).is_some_and(char::is_ascii_digit);
        if !joined {
            break;
        }
        i += 1;
    }
    if run.get(i).is_some_and(|&c| is_word(c)) {
        return None;
    }
    Some(i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sentence::NoLanguageModel;

    fn pieces(text: &str) -> Vec<String> {
        text_pieces(text, &NoLanguageModel)
    }

    #[test]
    fn pieces_join_back_to_the_original() {
        for text in [
            "fn main() {\n    run(1, 2);\n}\r\n",
            "请前往 mt.cn/15VOWoda 还车。Hi, OK?",
            "电话 +86 157-3760-5019，邮箱 a.b@mail.com\t\t日期 2026-10-19 10:43",
        ] {
            assert_eq!(pieces(text).concat(), text);
        }
    }

    #[test]
    fn whitespace_runs_and_newlines_are_pieces() {
        assert_eq!(
            pieces("if x {\n    y;\n}"),
            ["if", " ", "x", " ", "{", "\n", "    ", "y", ";", "\n", "}"]
        );
    }

    #[test]
    fn links_emails_and_numbers_stay_whole() {
        assert_eq!(
            pieces("(see https://qingjian.app/docs)"),
            ["(", "see", " ", "https://qingjian.app/docs", ")"]
        );
        assert_eq!(pieces("a.b@mail.com"), ["a.b@mail.com"]);
        assert_eq!(
            pieces("+86 157-3760-5019 2026-10-19 10:43 1,000.5 15737605019"),
            [
                "+86",
                " ",
                "157-3760-5019",
                " ",
                "2026-10-19",
                " ",
                "10:43",
                " ",
                "1,000.5",
                " ",
                "15737605019"
            ]
        );
    }

    #[test]
    fn repeated_operators_merge_but_brackets_do_not() {
        assert_eq!(
            pieces("a// b== c(("),
            ["a", "//", " ", "b", "==", " ", "c", "(", "("]
        );
        assert_eq!(pieces("3rd x-y"), ["3rd", " ", "x", "-", "y"]);
    }
}
