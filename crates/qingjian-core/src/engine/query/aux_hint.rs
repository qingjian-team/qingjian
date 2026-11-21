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

    /// 码段对不上任何码时，把它当成「首字要用哪个字」的提示。
    ///
    /// 先让词图把这段拼音读成整句/整词，再从里面挑首字正好是提示字的路径 —— `yanzhi;haiyan`
    /// 的 `haiyan` → 海盐 → 「盐」，词图里有 `盐值` 这条路径就挑出来；`kaicu;cuxi` 的
    /// `cuxi` → 粗细 → 「粗」，挑出 `开粗`。这些词词库里不一定有，只能靠词图拼。
    ///
    /// 词图里也没有时退回「含提示字的已有候选」，再没有就是空。
    pub(super) fn hinted_candidates(
        &self,
        scored: &[crate::ranking::Scored<'_>],
        best: &crate::parser::Segmentation,
        code: &str,
    ) -> Vec<Candidate> {
        let hints = self.hint_chars(code);
        if hints.is_empty() {
            return Vec::new();
        }
        // 一、词图：首字是提示字的路径排最前
        let mut items = Vec::new();
        let sentences = self.sentence_candidates(&mut items, best, false, 8);
        let mut pinned: Vec<Candidate> = sentences
            .into_iter()
            .filter(|candidate| {
                candidate
                    .text
                    .chars()
                    .next()
                    .is_some_and(|ch| hints.contains(&ch))
            })
            .collect();
        // 二、词图里没有就退回已有候选：词首是提示字的排前面，含提示字的跟后面
        if pinned.is_empty() {
            // 权分三档：本身就是那个单字 > 词首是它 > 只是含它，各档内保持原序
            let rank = |text: &str| -> Option<u8> {
                let mut chars = text.chars();
                let first = chars.next()?;
                if !hints.contains(&first) {
                    return text.chars().any(|ch| hints.contains(&ch)).then_some(2);
                }
                // 单字（只剩它自己）给的权最高：用户要的多半就是这个字
                Some(if chars.next().is_none() { 0 } else { 1 })
            };
            let mut order: Vec<(usize, u8)> = scored
                .iter()
                .enumerate()
                .filter_map(|(index, item)| rank(item.hit.text).map(|tier| (index, tier)))
                .collect();
            order.sort_by_key(|(index, tier)| (*tier, *index));
            pinned = order
                .into_iter()
                .map(|(index, _)| super::chinese_candidate(&scored[index], None))
                .collect();
        }
        pinned
    }
}
