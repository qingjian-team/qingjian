//! 算式结果补全：光标前刚上屏了一道算式（`1+2`、`3×4=`）时，候选给出结果。
//! 手机上数字与运算符是在数字键盘上直接上屏的，不经组句，所以按光标前的文字算，而不是像 `v1+2` 那样在缓冲区里算。

mod completion;

pub use self::completion::CalculationCompletion;

use crate::shortcut::{evaluate, expression_char};

/// 往前最多看多少个字符：再长就不像随手打的算式了。
const MAX_LEN: usize = 64;

/// 光标前的文字 `before` 以一道算式结尾时给出结果：
/// - 末尾是 `=`（`1+2=`）：一条，补上结果 `3`。
/// - 否则两条：`3` 把算式换成结果，`1+2=3` 在后面补上 `=3`。
///
/// 不是算式（单个数、前面紧挨着字母、像日期或电话号码的 `2026-10-19`、`0755-1234`）或算不了时为空。
pub fn calculation_completions(before: &str) -> Vec<CalculationCompletion> {
    let (body, equals) = match before.strip_suffix('=') {
        Some(body) => (body, true),
        None => (before, false),
    };
    let run: Vec<char> = body
        .chars()
        .rev()
        .take_while(|&c| expression_char(c).is_some())
        .take(MAX_LEN + 1)
        .collect();
    if run.is_empty() || run.len() > MAX_LEN {
        return Vec::new();
    }
    let expression: String = run.iter().rev().collect();
    let head = &body[..body.len() - expression.len()];
    if head
        .chars()
        .last()
        .is_some_and(|c| c.is_ascii_alphanumeric())
    {
        return Vec::new();
    }
    let normalized: String = expression.chars().filter_map(expression_char).collect();
    if !looks_like_arithmetic(&normalized) {
        return Vec::new();
    }
    let Some(result) = evaluate(&normalized) else {
        return Vec::new();
    };
    if equals {
        return vec![CalculationCompletion {
            label: result.clone(),
            insert: result,
            replace: 0,
        }];
    }
    vec![
        CalculationCompletion {
            label: result.clone(),
            insert: result.clone(),
            replace: run.len(),
        },
        CalculationCompletion {
            label: format!("{expression}={result}"),
            insert: format!("={result}"),
            replace: 0,
        },
    ]
}

/// 光标前最后一个字是 `c` 时可能是一道算式的结尾（数字、右括号、等号）：壳据此决定要不要来问结果。
pub fn may_end_calculation(c: char) -> bool {
    c.is_ascii_digit() || matches!(c, ')' | '=')
}

fn is_operator(c: char) -> bool {
    matches!(c, '+' | '-' | '*' | '/' | '^')
}

/// 至少有一个夹在两个数之间的运算符；只用 `-` 或只用 `/` 连了两次以上的是日期、电话号码，
/// 带前导零的数（`0755`）也是号码，都不算。
fn looks_like_arithmetic(text: &str) -> bool {
    let bytes = text.as_bytes();
    let binary: Vec<u8> = (1..bytes.len())
        .filter(|&i| {
            is_operator(char::from(bytes[i])) && matches!(bytes[i - 1], b'0'..=b'9' | b')')
        })
        .map(|i| bytes[i])
        .collect();
    if binary.is_empty() {
        return false;
    }
    let all_same = binary.iter().all(|&op| op == binary[0]);
    if binary.len() >= 2 && all_same && matches!(binary[0], b'-' | b'/') && !text.contains('(') {
        return false;
    }
    !text
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .any(|number| number.len() > 1 && number.starts_with('0') && !number.starts_with("0."))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(before: &str) -> Vec<String> {
        calculation_completions(before)
            .into_iter()
            .map(|completion| completion.label)
            .collect()
    }

    #[test]
    fn result_replaces_or_follows() {
        let completions = calculation_completions("总共1+2");
        assert_eq!(completions.len(), 2);
        assert_eq!(completions[0].insert, "3");
        assert_eq!(completions[0].replace, 3);
        assert_eq!(completions[1].label, "1+2=3");
        assert_eq!(completions[1].insert, "=3");
        assert_eq!(completions[1].replace, 0);
    }

    #[test]
    fn trailing_equals_gives_only_the_result() {
        assert_eq!(labels("3×4="), ["12"]);
        assert_eq!(calculation_completions("3×4=")[0].insert, "12");
        assert_eq!(labels("10÷4"), ["2.5", "10÷4=2.5"]);
    }

    #[test]
    fn not_arithmetic() {
        assert!(labels("123").is_empty());
        assert!(labels("1+").is_empty());
        assert!(labels("abc1+2").is_empty());
        assert!(labels("2026-10-19").is_empty());
        assert!(labels("2026/10/19").is_empty());
        assert!(labels("0755-1234").is_empty());
        assert!(labels("1/0").is_empty());
        assert!(labels("").is_empty());
    }

    #[test]
    fn results_only_after_possible_endings() {
        for before in ["1+2", "(1+2)", "1+2="] {
            assert!(before.chars().last().is_some_and(may_end_calculation));
            assert!(!labels(before).is_empty());
        }
        assert!(!may_end_calculation('+'));
        assert!(!may_end_calculation('×'));
    }

    #[test]
    fn mixed_operators_still_count() {
        assert_eq!(labels("10-2-3"), Vec::<String>::new());
        assert_eq!(labels("(10-2)-3"), ["5", "(10-2)-3=5"]);
        assert_eq!(labels("2*3-1"), ["5", "2*3-1=5"]);
    }
}
