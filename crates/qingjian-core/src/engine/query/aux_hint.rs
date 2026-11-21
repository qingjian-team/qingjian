//! 辅码态下码段一个码都对不上时的退路：把码段当成「要哪个字」的提示。
//!
//! 码段是码表里的码，可用户常常敲的是别的意思：`fuma;fuzhu` 想说的是「辅」也在「辅助」里，
//! `fuma;chefu` 想说的是「辅」＝车＋甫。两种都当作提示，给含那些字的候选加权 ——
//! 不是过滤掉其余的，只是把它们挪到前面，免得把一个猜错的意思当成铁律。
//!
//! - **拆字**：`chefu` → 车 + 甫 → 辅（与 `u` 模式同一套拆字表）
//! - **组词**：`fuzhu` → 辅助 → 辅、助（码段在任意位置切两段，第一个查得到词的切法就用它）
//!
//! 两条都试过还是没有字，就返回空，由调用方保持原来的空候选。

use super::Engine;
use crate::candidate::Candidate;

/// 提示能给出的字数上限：太多了就不是提示，是把半个字库搬出来。
const HINT_LIMIT: usize = 8;

impl Engine {
    /// 码段指向的字：先按拆字找，再按「组词」找。
    pub(crate) fn hint_chars(&self, code: &str) -> Vec<char> {
        let mut chars: Vec<char> = Vec::new();
        if code.chars().count() < 2 {
            return chars;
        }
        for hit in crate::chaizi::lookup(code, HINT_LIMIT) {
            if !chars.contains(&hit.ch) {
                chars.push(hit.ch);
            }
        }
        if chars.len() < HINT_LIMIT {
            for (at, _) in code.char_indices().skip(1) {
                let (head, tail) = code.split_at(at);
                if head.chars().count() < 2 || tail.is_empty() {
                    continue;
                }
                let words = self.dictionary.lookup(&[head, tail], false);
                if words.is_empty() {
                    continue;
                }
                for word in words.iter().take(3) {
                    for ch in word.text.chars() {
                        if chars.len() < HINT_LIMIT && !chars.contains(&ch) {
                            chars.push(ch);
                        }
                    }
                }
                // 第一个查得到词的切法就够：再切下去只是同一批字的另一种读法
                break;
            }
        }
        chars
    }

    /// 码段对不上任何码时，按提示把候选挪到前面：词首就是那个字的排最前，其余保持原序。
    pub(super) fn hinted_candidates(
        &self,
        scored: &[crate::ranking::Scored<'_>],
        code: &str,
    ) -> Vec<Candidate> {
        let hints = self.hint_chars(code);
        if hints.is_empty() {
            return Vec::new();
        }
        let mut order: Vec<(usize, bool)> = scored
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let text = item.hit.text;
                let leads = text.chars().next().is_some_and(|ch| hints.contains(&ch));
                if leads {
                    Some((index, true))
                } else if text.chars().any(|ch| hints.contains(&ch)) {
                    Some((index, false))
                } else {
                    None
                }
            })
            .collect();
        order.sort_by_key(|(index, leads)| (!*leads, *index));
        order
            .into_iter()
            .map(|(index, _)| super::chinese_candidate(&scored[index], None))
            .collect()
    }
}
