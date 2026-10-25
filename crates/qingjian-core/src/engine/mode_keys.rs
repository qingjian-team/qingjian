//! 前缀模式键：表达式 / 问字模式的入口字母，以及 `?` 是否也当问字入口。

use serde::{Deserialize, Serialize};

use crate::shortcut::EXPRESSION_PREFIX;

/// 问字模式的标点入口：[`ModeKeys::question_mark`] 打开时任何配置下 `?` 开头都进问字模式，也是英文模式下唯一的入口。
pub const QUESTION_PREFIX: char = '?';

/// 快捷短语模式前缀：`i` 开头匹配自定义短语表。
pub const SHORTCUT_PREFIX: char = 'i';

/// 前缀模式键，配置文件 `[shortcut]` 分节。
///
/// 搜狗 / 微软那一家的做法：用不能开头拼任何音节的字母（`v` `u` `i`）一键进模式，
/// 不要修饰键，中文模式下零冲突。缺省 `v` 表达式、`u` 问字（含 Unicode 码点）、`i` 快捷短语。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModeKeys {
    /// 表达式模式前缀：`v1+2`、`v123`。
    pub expression: char,

    /// 问字模式前缀：`usangemu`（三个木）、`u4e00`（码点）。
    pub question: char,

    /// 快捷短语模式前缀：`i` 开头匹配自定义短语表，如 `iemail` → 用户邮箱。
    pub shortcut: char,

    /// `?` 开头是否也进问字模式。缺省关：没在组句时敲的问号就是问号；
    /// 开着时缓冲区为空敲 `?` 先进问字，后面跟字母才是问题，跟别的键还原成问号。
    pub question_mark: bool,
}

impl Default for ModeKeys {
    fn default() -> Self {
        Self {
            expression: EXPRESSION_PREFIX,
            question: 'u',
            shortcut: 'i',
            question_mark: false,
        }
    }
}

impl ModeKeys {
    /// 能当模式键的字母：不是任何拼音音节的开头。
    pub const CANDIDATES: [char; 3] = ['v', 'u', 'i'];

    /// 双拼下 v / u / i 都是音节键，模式键换成对应的大写字母（Shift+V / Shift+U，搜狗 / 微软的做法）。
    pub fn shifted(self) -> Self {
        Self {
            expression: self.expression.to_ascii_uppercase(),
            question: self.question.to_ascii_uppercase(),
            shortcut: self.shortcut.to_ascii_uppercase(),
            question_mark: self.question_mark,
        }
    }

    /// 去掉字母模式键：形码下每个字母都是字根键，大写也让位，只剩 `?`（开着的话）进问字。
    pub fn letterless(self) -> Self {
        Self {
            expression: '\0',
            question: '\0',
            shortcut: '\0',
            question_mark: self.question_mark,
        }
    }

    /// 三个键都合法且互不相同。不合法的配置整个退回缺省，不做一半。
    pub fn is_valid(&self) -> bool {
        self.expression != self.question
            && self.expression != self.shortcut
            && self.question != self.shortcut
            && Self::CANDIDATES.contains(&self.expression)
            && Self::CANDIDATES.contains(&self.question)
            && Self::CANDIDATES.contains(&self.shortcut)
    }

    /// 非法配置退回缺省（`?` 开关照旧保留）。
    pub fn sanitized(self) -> Self {
        if self.is_valid() {
            self
        } else {
            Self {
                question_mark: self.question_mark,
                ..Self::default()
            }
        }
    }

    pub fn is_expression(&self, input: &str, zhuyin: bool) -> bool {
        input.starts_with(self.expression)
            && (!zhuyin || crate::zhuyin::layout::map_key(self.expression).is_none())
    }

    pub fn is_question(&self, input: &str, zhuyin: bool) -> bool {
        (input.starts_with(self.question)
            && (!zhuyin || crate::zhuyin::layout::map_key(self.question).is_none()))
            || self.is_question_mark(input)
    }

    /// 快捷短语模式：`i` 开头，匹配自定义短语表。
    pub fn is_shortcut(&self, input: &str, zhuyin: bool) -> bool {
        input.starts_with(self.shortcut)
            && (!zhuyin || crate::zhuyin::layout::map_key(self.shortcut).is_none())
    }

    /// 快捷短语模式下前缀之后的部分。不在快捷短语模式时原样返回。
    pub fn shortcut_body<'a>(&self, input: &'a str, zhuyin: bool) -> &'a str {
        if self.is_shortcut(input, zhuyin) {
            &input[self.shortcut.len_utf8()..]
        } else {
            input
        }
    }

    /// 是否以 `?` 进的问字模式：开关关着时 `?` 不是入口。
    fn is_question_mark(&self, input: &str) -> bool {
        self.question_mark && input.starts_with(QUESTION_PREFIX)
    }

    /// 问字模式下前缀之后的部分。不在问字模式时原样返回。
    pub fn question_body<'a>(&self, input: &'a str, zhuyin: bool) -> &'a str {
        if self.is_question_mark(input) {
            &input[QUESTION_PREFIX.len_utf8()..]
        } else if self.is_question(input, zhuyin) {
            &input[self.question.len_utf8()..]
        } else {
            input
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_v_and_u_and_question_mark_is_off() {
        let keys = ModeKeys::default();
        assert!(keys.is_expression("v12", false));
        assert!(keys.is_question("usangemu", false));
        assert!(!keys.is_question("?sangemu", false));
        assert_eq!(keys.question_body("usangemu", false), "sangemu");
        assert_eq!(keys.question_body("?sangemu", false), "?sangemu");
        assert!(!keys.is_question("nihao", false));
    }

    #[test]
    fn question_mark_is_an_alias_only_when_switched_on() {
        let keys = ModeKeys {
            question_mark: true,
            ..ModeKeys::default()
        };
        assert!(keys.is_question("?sangemu", false));
        assert_eq!(keys.question_body("?sangemu", false), "sangemu");
        // 双拼下字母键换成大写，`?` 开关跟着走
        let shifted = keys.shifted();
        assert!(!shifted.is_question("usangemu", false));
        assert!(shifted.is_question("Usangemu", false));
        assert!(shifted.is_expression("V1+2", false));
        assert_eq!(shifted.question_body("Usangemu", false), "sangemu");
        assert!(shifted.is_question("?sangemu", false));
        assert!(!ModeKeys::default().shifted().is_question("?x", false));
    }

    #[test]
    fn letterless_leaves_only_the_question_mark() {
        let keys = ModeKeys {
            question_mark: true,
            ..ModeKeys::default()
        };
        let letterless = keys.letterless();
        assert!(!letterless.is_expression("v1+2", false));
        assert!(!letterless.is_question("usangemu", false));
        assert!(letterless.is_question("?sangemu", false));
        assert_eq!(letterless.question_body("?sangemu", false), "sangemu");
        assert!(!ModeKeys::default().letterless().is_question("?x", false));
    }

    #[test]
    fn invalid_combinations_fall_back_to_defaults() {
        let same = ModeKeys {
            expression: 'v',
            question: 'v',
            shortcut: 'v',
            question_mark: true,
        };
        assert!(!same.is_valid());
        assert_eq!(
            same.sanitized(),
            ModeKeys {
                question_mark: true,
                ..ModeKeys::default()
            }
        );
        let pinyin_initial = ModeKeys {
            expression: 'v',
            question: 'z',
            shortcut: 'i',
            question_mark: false,
        };
        assert_eq!(pinyin_initial.sanitized(), ModeKeys::default());
        let swapped = ModeKeys {
            expression: 'i',
            question: 'v',
            shortcut: 'u',
            question_mark: false,
        };
        assert!(swapped.is_valid());
        assert!(swapped.is_question("v4e00", false));
    }

    #[test]
    fn deserializes_from_single_character_strings() {
        let keys: ModeKeys =
            toml::from_str("expression = \"i\"\nquestion = \"u\"\nquestion_mark = true\n").unwrap();
        assert_eq!(keys.expression, 'i');
        assert_eq!(keys.question, 'u');
        assert_eq!(keys.shortcut, 'i');
        assert!(keys.question_mark);
        let old: ModeKeys = toml::from_str("expression = \"v\"\nquestion = \"u\"\n").unwrap();
        assert!(!old.question_mark);
    }

    #[test]
    fn shortcut_mode_is_detected() {
        let keys = ModeKeys::default();
        assert!(keys.is_shortcut("iemail", false));
        assert_eq!(keys.shortcut_body("iemail", false), "email");
        assert!(!keys.is_shortcut("usangemu", false));
        assert!(!keys.is_shortcut("nihao", false));
    }

    #[test]
    fn shortcut_mode_shifted_on_double_pinyin() {
        let keys = ModeKeys::default().shifted();
        assert!(keys.is_shortcut("Iemail", false));
        assert_eq!(keys.shortcut_body("Iemail", false), "email");
        assert!(!keys.is_shortcut("usangemu", false));
    }

    #[test]
    fn shortcut_mode_is_invalid_with_duplicate_keys() {
        let duplicate = ModeKeys {
            expression: 'v',
            question: 'u',
            shortcut: 'v',
            question_mark: false,
        };
        assert!(!duplicate.is_valid());
        let sanitized = duplicate.sanitized();
        assert_eq!(sanitized.shortcut, 'i');
    }
}
