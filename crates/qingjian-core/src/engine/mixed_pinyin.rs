//! 双拼兼容全拼：挑选显示读法，并按候选音节找回它对应的原始按键。

use std::cmp::Reverse;

use crate::candidate::Candidate;
use crate::engine::{Engine, choice_key};
use crate::shuangpin::Decoded;
use qingjian_dictionary::canonical_syllable;

impl Engine {
    pub(super) fn mixed_decodings(&self, keys: &str) -> Vec<Decoded> {
        let Some(scheme) = self.shuangpin else {
            return Vec::new();
        };
        let mut variants = scheme.decode_mixed(keys);
        // 能覆盖整段的已知读法先显示；同覆盖长度仍优先原来的双拼读法。
        variants.sort_by_cached_key(|decoded| {
            let Some(segmentation) = decoded.segmentation() else {
                return Reverse((0, false));
            };
            let patterns = segmentation.patterns();
            let expanded = self.fuzzy.expand(&patterns);
            let positions = expanded.positions();
            let mut coverage = 0;
            for end in 1..=patterns.len() {
                if !self.lookup_exact_all(&positions[..end]).is_empty() {
                    let len = segmentation.syllables[..end]
                        .iter()
                        .map(|s| s.text.len())
                        .sum::<usize>()
                        + end
                        - 1;
                    coverage = decoded.keys_for(len);
                }
            }
            if let Some(conversion) = self.convert_sentence(&patterns, false)
                && !conversion.has_placeholder()
            {
                coverage = decoded.keys_for(decoded.pinyin().len());
            }
            Reverse((coverage, decoded.is_complete()))
        });
        variants
    }

    /// 匹配只接受原音、规范音、模糊音和末尾前缀；不能拿另一条读法的敲错对齐消耗键。
    pub(super) fn mixed_match(
        &self,
        decoded: &Decoded,
        candidate: &Candidate,
    ) -> Option<(usize, String)> {
        let units: Vec<_> = decoded
            .units()
            .iter()
            .filter(|u| !u.is_separator())
            .collect();
        let count = units.len().min(candidate.syllables.len());
        if count == 0 {
            return None;
        }
        for (index, (unit, syllable)) in units.iter().zip(&candidate.syllables).enumerate() {
            let exact = canonical_syllable(&unit.pinyin) == canonical_syllable(syllable);
            let partial =
                !unit.complete && index + 1 == units.len() && syllable.starts_with(&unit.pinyin);
            if !exact && !partial && !self.fuzzy.is_variant(&unit.pinyin, syllable) {
                return None;
            }
        }
        let len = units[..count].iter().map(|u| u.pinyin.len()).sum::<usize>() + count - 1;
        Some((decoded.keys_for(len), choice_key(decoded.pinyin(), len)))
    }

    pub(super) fn mixed_unit_len(&self, keys: &str, backwards: bool) -> Option<usize> {
        if !self.shuangpin_full_pinyin || self.shuangpin.is_none() || self.zhuyin {
            return None;
        }
        let decoded = self.mixed_decodings(keys).into_iter().next()?;
        if backwards && !decoded.tail().is_empty() {
            return Some(decoded.tail().len());
        }
        if !backwards && decoded.units().is_empty() {
            return None;
        }
        let mut len = 0;
        let units: Box<dyn Iterator<Item = _>> = if backwards {
            Box::new(decoded.units().iter().rev())
        } else {
            Box::new(decoded.units().iter())
        };
        for unit in units {
            len += unit.keys.len();
            if !unit.is_separator() {
                break;
            }
        }
        Some(len)
    }
}
