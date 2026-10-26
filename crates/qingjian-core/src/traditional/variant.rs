//! 繁体输出的地区变体。

use serde::{Deserialize, Serialize};

/// 候选与上屏文本要不要转成繁体，转成哪一地的写法。
///
/// 不做成布尔是因为台湾与香港的字形取舍不同（裡 / 裏、著 / 着）；一个开关表达不了这三种结果。
/// 三档都只换字形不换用语（软件 → 軟件），输入法不替用户改词。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TraditionalVariant {
    /// 不转换，输出简体。
    #[default]
    Off,

    /// 台湾字形。OpenCC 的 `s2tw`，与 0.2.1 的 `traditional = true` 相同。
    Taiwan,

    /// 香港繁体。OpenCC 的 `s2hk`。
    Hongkong,

    /// 通用繁体，只换字形不换用语。OpenCC 的 `s2t`。
    Standard,
}

impl TraditionalVariant {
    /// 全部取值，设置界面按这个顺序列出。
    pub const ALL: [Self; 4] = [Self::Off, Self::Taiwan, Self::Hongkong, Self::Standard];

    /// 配置文件里的写法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Taiwan => "taiwan",
            Self::Hongkong => "hongkong",
            Self::Standard => "standard",
        }
    }

    /// 界面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "简体",
            Self::Taiwan => "繁体（台湾）",
            Self::Hongkong => "繁体（香港）",
            Self::Standard => "繁体（通用字形）",
        }
    }

    /// 只说字形的短名，用在已经写明「繁体」的地方（手机键盘设置的「繁体字形」）。
    pub fn short_label(self) -> &'static str {
        match self {
            Self::Off => "简体",
            Self::Taiwan => "台湾",
            Self::Hongkong => "香港",
            Self::Standard => "通用",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 配置里的写法与枚举一一对上，设置界面按 `key` 存、按 `ALL` 列。
    #[test]
    fn keys_round_trip_through_toml() {
        for variant in TraditionalVariant::ALL {
            let toml = format!("value = \"{}\"", variant.key());
            #[derive(serde::Deserialize)]
            struct Holder {
                value: TraditionalVariant,
            }
            let holder: Holder = toml::from_str(&toml).expect("解析配置里的写法");
            assert_eq!(holder.value, variant);
        }
    }

    /// 缺省不转换：没写这一项的老配置照旧出简体。
    #[test]
    fn default_is_off() {
        assert_eq!(TraditionalVariant::default(), TraditionalVariant::Off);
    }
}
