//! 词库查询：对外的查词入口与词目枚举。

use qingjian_format::Metadata;

use crate::matching::Match;
use crate::pattern::SyllablePattern;

use super::Dictionary;

impl Dictionary {
    /// 全部词目，按拼音键的字节序、同一个键下按词频降序。给反查（汉字 → 读音）建索引用。
    pub fn entries(&self) -> impl Iterator<Item = Match<'_>> + '_ {
        self.index.iter().flat_map(move |entry| {
            let key = self.key(entry);
            let range =
                entry.first_slot as usize..entry.first_slot as usize + entry.slot_count as usize;
            range.map(move |position| {
                let slot = &self.slots[position];
                Match {
                    text: self.text(slot),
                    pinyin: key,
                    frequency: slot.frequency,
                    reading_total: self.reading_total(key, self.text(slot)),
                    tones: self.tones(position, slot),
                    secondary: self.secondary,
                    exact: true,
                }
            })
        })
    }

    /// 只查音节数与模式长度**恰好相等**的词：整句转换的词图每个格子只要正好覆盖这几个音节的词，
    /// 输入前缀出候选也只要这种。与 [`Self::lookup_pattern`] 同一套收窄，只是最后一级不收更长的词、
    /// 简拼位置只看每个音节块的第一条键，代价与匹配到的音节组合数成正比。
    pub fn lookup_exact(&self, pattern: &[SyllablePattern<'_>]) -> Vec<Match<'_>> {
        let positions: Vec<&[SyllablePattern<'_>]> =
            pattern.iter().map(std::slice::from_ref).collect();
        self.lookup_positions(&positions, true)
    }

    /// 按音节序列查词。
    ///
    /// - 完整音节：命中音节序列相同的词，以及以该序列为前缀的更长词。
    /// - `last_is_partial` 为真时，最后一个音节按前缀匹配（如 `kai f` 命中 `kai fa`、`kai fang`）。
    pub fn lookup(&self, syllables: &[&str], last_is_partial: bool) -> Vec<Match<'_>> {
        let count = syllables.len();
        let pattern: Vec<SyllablePattern<'_>> = syllables
            .iter()
            .enumerate()
            .map(|(i, s)| SyllablePattern {
                text: s,
                complete: !(last_is_partial && i + 1 == count),
            })
            .collect();
        self.lookup_pattern(&pattern)
    }

    /// 按模式查词：每个位置可以是完整音节，也可以是前缀 / 声母（简拼）。
    /// 命中音节数 ≥ 模式长度、逐位满足模式的词。
    pub fn lookup_pattern(&self, pattern: &[SyllablePattern<'_>]) -> Vec<Match<'_>> {
        let positions: Vec<&[SyllablePattern<'_>]> =
            pattern.iter().map(std::slice::from_ref).collect();
        self.lookup_positions(&positions, false)
    }

    /// 每个位置给多种写法（模糊音：`zi` 也接受 `zhi`），命中音节数 ≥ 位置数、逐位满足其中一种写法的词。
    ///
    /// 同一位置的写法之间不能互相覆盖（前缀 `z` 已包含前缀 `zh`，两者不要同时给；同一写法不要重复），
    /// 否则命中会重复出现。调用方按这个契约生成写法，这里不去重。
    pub fn lookup_pattern_alt<'p>(
        &self,
        positions: &[impl AsRef<[SyllablePattern<'p>]>],
    ) -> Vec<Match<'_>> {
        self.lookup_positions(positions, false)
    }

    /// [`Self::lookup_exact`] 的多写法版。
    pub fn lookup_exact_alt<'p>(
        &self,
        positions: &[impl AsRef<[SyllablePattern<'p>]>],
    ) -> Vec<Match<'_>> {
        self.lookup_positions(positions, true)
    }

    fn lookup_positions<'p>(
        &self,
        positions: &[impl AsRef<[SyllablePattern<'p>]>],
        exact_only: bool,
    ) -> Vec<Match<'_>> {
        let mut matches = Vec::new();
        if positions.is_empty() || positions.iter().any(|p| p.as_ref().is_empty()) {
            return matches;
        }
        let mut prefix = String::with_capacity(positions.len() * 6);
        self.narrow(
            positions,
            0,
            &mut prefix,
            0..self.index.len(),
            exact_only,
            &mut matches,
        );
        matches
    }

    /// `.qj` 里的来历；TSV 解析的返回 `None`。
    pub fn metadata(&self) -> Option<&Metadata> {
        self.metadata.as_ref()
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// 全部词频之和。
    pub fn total_frequency(&self) -> u64 {
        self.total_frequency
    }
}
