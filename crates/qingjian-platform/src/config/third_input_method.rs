//! 连按两次 Shift 切换到的第三输入法（`[general] double_shift_input`）。

use serde::{Deserialize, Serialize};

/// 连按两次 Shift 的目标输入法。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThirdInputMethod {
    /// 不启用第三输入法切换。
    #[default]
    Off,

    /// 越南语输入法。
    Vietnamese,
}

impl ThirdInputMethod {
    /// 设置界面中的排列顺序。
    pub const ALL: [Self; 2] = [Self::Off, Self::Vietnamese];

    /// 配置文件里的写法。
    pub const fn key(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Vietnamese => "vietnamese",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ThirdInputMethod;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Value {
        method: ThirdInputMethod,
    }

    #[test]
    fn default_is_off_and_keys_round_trip() {
        assert_eq!(ThirdInputMethod::default(), ThirdInputMethod::Off);
        assert_eq!(ThirdInputMethod::Off.key(), "off");
        assert_eq!(ThirdInputMethod::Vietnamese.key(), "vietnamese");
        assert_eq!(
            toml::from_str::<Value>("method = \"vietnamese\"")
                .unwrap()
                .method,
            ThirdInputMethod::Vietnamese
        );
    }
}
