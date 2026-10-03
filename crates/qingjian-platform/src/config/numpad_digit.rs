//! Windows 中文组句中小键盘数字的用途，供配置与设置界面共用。

use serde::{Deserialize, Serialize};

/// `[general] numpad_digit`；其他平台不使用这项设置。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NumpadDigit {
    /// 选择当前页候选，延续现有行为。
    #[default]
    Select,

    /// 作为内容进入组句缓冲区。
    Direct,
}

impl NumpadDigit {
    /// 设置界面的选项顺序。
    pub const ALL: [Self; 2] = [Self::Select, Self::Direct];

    /// 配置文件中的写法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Select => "select",
            Self::Direct => "direct",
        }
    }

    /// 设置界面的名称。
    pub fn label(self) -> &'static str {
        match self {
            Self::Select => "选择候选",
            Self::Direct => "直接输入数字",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::NumpadDigit;
    use crate::Config;

    #[test]
    fn legacy_config_keeps_selection_and_both_values_round_trip() {
        let legacy: Config = toml::from_str("[general]\npage_size = 5\n").unwrap();
        assert_eq!(legacy.general.numpad_digit, NumpadDigit::Select);
        for mode in NumpadDigit::ALL {
            let text = format!("[general]\nnumpad_digit = \"{}\"\n", mode.key());
            let config: Config = toml::from_str(&text).unwrap();
            assert_eq!(config.general.numpad_digit, mode);
            let restored: Config = toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
            assert_eq!(restored.general.numpad_digit, mode);
        }
        assert!(toml::from_str::<Config>("[general]\nnumpad_digit = \"invalid\"").is_err());
    }
}
