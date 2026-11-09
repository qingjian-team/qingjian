//! 多音字全部读音的词频之和，给 [`crate::Match::reading_share`] 用：语言模型只按字形给分，
//! 少见读音（`区 ou`）不折算会拿到常见读音的分。

use std::collections::HashMap;

use super::Dictionary;

/// 单字 → 它全部读音的词频之和。只收不止一个读音的字：一个读音的占比总是 1，不必记。
#[derive(Debug, Default)]
pub(super) struct Readings {
    totals: HashMap<Box<str>, u32>,
}

impl Readings {
    /// 只走单音节的键（键里没有空格），多字的（儿化的 `哪儿 nar`）跳过。
    pub(super) fn build(dictionary: &Dictionary) -> Self {
        let mut seen: HashMap<&str, (u32, u32)> = HashMap::new();
        for entry in dictionary.index.iter() {
            if dictionary.key(entry).contains(' ') {
                continue;
            }
            let first = entry.first_slot as usize;
            for slot in &dictionary.slots[first..first + entry.slot_count as usize] {
                let text = dictionary.text(slot);
                if text.chars().nth(1).is_some() {
                    continue;
                }
                let (total, readings) = seen.entry(text).or_default();
                *total = total.saturating_add(slot.frequency);
                *readings += 1;
            }
        }
        let totals = seen
            .into_iter()
            .filter(|(_, (_, readings))| *readings > 1)
            .map(|(text, (total, _))| (text.into(), total))
            .collect();
        Self { totals }
    }

    /// `text` 全部读音的词频之和；不是多音字时是 0。
    pub(super) fn total(&self, text: &str) -> u32 {
        self.totals.get(text).copied().unwrap_or(0)
    }
}
