use serde::{Deserialize, Serialize};

use crate::shortcut::EXPRESSION_PREFIX;

/// 问字模式的通用别名：任何配置下 `?` 开头都进问字模式，也是英文模式下唯一的入口。
pub const QUESTION_PREFIX: char = '?';

/// 前缀模式键，配置文件 `[shortcut]` 分节。
///
/// 搜狗 / 微软那一家的做法：用不能开头拼任何音节的字母（`v` `u` `i`）一键进模式，
/// 不要修饰键，中文模式下零冲突。缺省 `v` 表达式、`u` 问字（含 Unicode 码点）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModeKeys {
    /// 表达式模式前缀：`v1+2`、`v123`。
    pub expression: char,

    /// 问字模式前缀：`usangemu`（三个木）、`u4e00`（码点）。`?` 永远是别名。
    pub question: char,
}

impl Default for ModeKeys {
    fn default() -> Self {
        Self {
            expression: EXPRESSION_PREFIX,
            question: 'u',
        }
    }
}

impl ModeKeys {
    /// 能当模式键的字母：不是任何拼音音节的开头。
    pub const CANDIDATES: [char; 3] = ['v', 'u', 'i'];

    /// 没有字母模式键：双拼下 v / u / i 都是音节键，只剩 `?` 别名进问字。
    pub const LETTERLESS: Self = Self {
        expression: '\0',
        question: '\0',
    };

    /// 两个键都合法且互不相同。不合法的配置整个退回缺省，不做一半。
    pub fn is_valid(&self) -> bool {
        self.expression != self.question
            && Self::CANDIDATES.contains(&self.expression)
            && Self::CANDIDATES.contains(&self.question)
    }

    /// 非法配置退回缺省。
    pub fn sanitized(self) -> Self {
        if self.is_valid() {
            self
        } else {
            Self::default()
        }
    }

    pub fn is_expression(&self, input: &str) -> bool {
        input.starts_with(self.expression)
    }

    pub fn is_question(&self, input: &str) -> bool {
        input.starts_with(self.question) || input.starts_with(QUESTION_PREFIX)
    }

    /// 问字模式下前缀之后的部分。不在问字模式时原样返回。
    pub fn question_body<'a>(&self, input: &'a str) -> &'a str {
        input
            .strip_prefix(self.question)
            .or_else(|| input.strip_prefix(QUESTION_PREFIX))
            .unwrap_or(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_v_and_u_and_question_mark_is_always_an_alias() {
        let keys = ModeKeys::default();
        assert!(keys.is_expression("v12"));
        assert!(keys.is_question("usangemu"));
        assert!(keys.is_question("?sangemu"));
        assert_eq!(keys.question_body("usangemu"), "sangemu");
        assert_eq!(keys.question_body("?sangemu"), "sangemu");
        assert!(!keys.is_question("nihao"));
    }

    #[test]
    fn invalid_combinations_fall_back_to_defaults() {
        let same = ModeKeys {
            expression: 'v',
            question: 'v',
        };
        assert!(!same.is_valid());
        assert_eq!(same.sanitized(), ModeKeys::default());
        let pinyin_initial = ModeKeys {
            expression: 'v',
            question: 'z',
        };
        assert_eq!(pinyin_initial.sanitized(), ModeKeys::default());
        let swapped = ModeKeys {
            expression: 'i',
            question: 'v',
        };
        assert!(swapped.is_valid());
        assert!(swapped.is_question("v4e00"));
    }

    #[test]
    fn deserializes_from_single_character_strings() {
        let keys: ModeKeys = toml::from_str("expression = \"i\"\nquestion = \"u\"\n").unwrap();
        assert_eq!(keys.expression, 'i');
        assert_eq!(keys.question, 'u');
    }
}
