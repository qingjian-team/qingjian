//! 五笔连续上屏自动组词的开关和最大汉字数。

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AutoPhraseConfig {
    /// 是否学习连续上屏的五笔新词，默认开启；不删除已有词或关闭词频学习。
    pub enabled: bool,
    /// 五笔组词的最大汉字数，默认 4；读取时限制在 2～10。
    #[serde(deserialize_with = "max_chars")]
    pub max_chars: usize,
}

impl Default for AutoPhraseConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_chars: 4,
        }
    }
}

fn max_chars<'de, D: Deserializer<'de>>(deserializer: D) -> Result<usize, D::Error> {
    Ok(i64::deserialize(deserializer)?.clamp(2, 10) as usize)
}

#[cfg(test)]
mod tests {
    use super::AutoPhraseConfig;
    use crate::Config;

    #[test]
    fn defaults_and_bounds() {
        assert_eq!(
            Config::default().auto_phrase,
            AutoPhraseConfig {
                enabled: true,
                max_chars: 4
            }
        );
        for (raw, expected) in [(-1, 2), (0, 2), (2, 2), (4, 4), (10, 10), (11, 10)] {
            let config: Config = toml::from_str(&format!(
                "[auto_phrase]\nenabled = false\nmax_chars = {raw}"
            ))
            .unwrap();
            assert!(!config.auto_phrase.enabled);
            assert_eq!(config.auto_phrase.max_chars, expected);
        }
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config.auto_phrase, AutoPhraseConfig::default());
    }
}
