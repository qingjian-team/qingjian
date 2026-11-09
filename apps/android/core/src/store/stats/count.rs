//! 一段时间里的汉字数与英文词数。

use qingjian_core::Usage;
use serde::Serialize;

#[derive(Debug, Default, Serialize)]
pub struct StatsCount {
    pub hanzi: u64,

    pub english: u64,
}

impl From<Usage> for StatsCount {
    fn from(usage: Usage) -> Self {
        Self {
            hanzi: usage.hanzi,
            english: usage.english_words,
        }
    }
}
