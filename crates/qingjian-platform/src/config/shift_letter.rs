use serde::{Deserialize, Serialize};

use crate::i18n::tr;

/// 中文模式下按住 Shift 敲的字母怎么处理（`[general] shift_letter`）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShiftLetter {
    /// 拼音原样上屏、这个字母交给应用——两个平台原本的行为。缺省：换掉老用户的习惯得让人自己选。
    #[default]
    Passthrough,

    /// 字母进组句缓冲区，按小写参与匹配、原样上屏（回车 / 无候选）时还原大写，
    /// 所以 `Cpan` 与 `cpan` 一样出「C盘」。英文模式与英文直输段保持原样大小写。
    Compose,
}

impl ShiftLetter {
    /// 全部取值，设置界面按这个顺序列出。
    pub const ALL: [Self; 2] = [Self::Passthrough, Self::Compose];

    /// 配置文件里的写法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Passthrough => "passthrough",
            Self::Compose => "compose",
        }
    }

    /// 界面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Passthrough => tr("shift_letter.passthrough"),
            Self::Compose => tr("shift_letter.compose"),
        }
    }

    /// 会不会把大写字母收进组句缓冲区。
    pub fn compose(self) -> bool {
        matches!(self, Self::Compose)
    }
}

#[cfg(test)]
mod tests {
    use super::ShiftLetter;

    #[test]
    fn keys_round_trip_through_lowercase() {
        assert_eq!(ShiftLetter::default(), ShiftLetter::Passthrough);
        assert_eq!(ShiftLetter::Passthrough.key(), "passthrough");
        assert_eq!(ShiftLetter::Compose.key(), "compose");
        assert!(ShiftLetter::Compose.compose());
        assert!(!ShiftLetter::Passthrough.compose());
    }
}
