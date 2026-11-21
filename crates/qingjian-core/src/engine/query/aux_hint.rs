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
    /// 码段指向的字：先按「组词」找，再按拆字找。
    ///
    /// 顺序有讲究：拆字对 `zhenge` 这种会捞一堆不相干的字（可 / 顶 / 政 / 承…），
    /// 而组词给的是「整个 → 整」这种真正想要的字。组词那条排前面，拆字只作补充。
    pub(crate) fn hint_chars(&self, code: &str) -> Vec<char> {
        let mut chars: Vec<char> = Vec::new();
        if code.chars().count() < 2 {
            return chars;
        }
        // 一、组词：码段在任意位置切两段，第一个查得到词的切法就用它
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
        // 二、拆字：`chefu` → 车 + 甫 → 辅
        for hit in crate::chaizi::lookup(code, HINT_LIMIT) {
            if chars.len() >= HINT_LIMIT {
                break;
            }
            if !chars.contains(&hit.ch) {
                chars.push(hit.ch);
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
        // 有组词结果就只认它给出的字：拆字对 zhenge 会凑出 政 这类不相干的字，
        // 它们要是进了提示，政 这种单字就会被顶到最前面。
        let pairs_for_hint = self.hint_pairs(code);
        let hints: Vec<char> = if pairs_for_hint.is_empty() {
            self.hint_chars(code)
        } else {
            let mut unique: Vec<char> = Vec::new();
            for (_, ch) in &pairs_for_hint {
                if !unique.contains(ch) {
                    unique.push(*ch);
                }
            }
            unique
        };
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
                // 只认「这个词以提示字开头」：再退一步的「只是含它」会把 政治 这种
                // 只因含了个拆字凑出来的 政 就顶上去，用户看着莫名其妙。
                if !hints.contains(&first) {
                    return None;
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
        // 三、都没有就按音节拼一个：每个音节取最常见单字，提示字那位换成提示字。
        //     `kaicu;cuxi` 的 cu 用 粗、kai 用 开，拼出「开粗」。
        if pinned.is_empty()
            && let Some(built) = self.build_from_hints(code, best)
        {
            pinned.push(built);
        }
        pinned
    }

    /// 提示字与它对应的音节：`cuxi` → (cu, 粗)、(xi, 细)。
    ///
    /// 带音节是因为 `kaicu;cuxi` 里要的是「cu 这个音节写粗」，而 粗 在 `kaicu` 里是第二个
    /// 音节 —— 只看「词首是提示字」会把 `开粗` 漏掉。
    pub(crate) fn hint_pairs(&self, code: &str) -> Vec<(String, char)> {
        let mut pairs: Vec<(String, char)> = Vec::new();
        // 切成 2 段、3 段、4 段都试：`jiaru` 是 2 段（加入），`chengge` 也可能该切 2 段，
        // 但用户敲的组词常常是三个字以上，只切两刀会漏掉大半。
        let boundaries: Vec<usize> = code
            .char_indices()
            .map(|(at, _)| at)
            .filter(|at| *at > 0)
            .collect();
        let mut parts: Vec<Vec<&str>> = Vec::new();
        for (index, first) in boundaries.iter().enumerate() {
            if *first >= code.len() {
                continue;
            }
            // 两段
            parts.push(vec![&code[..*first], &code[*first..]]);
            // 三段、四段：在后半段里再切一刀 / 两刀
            for (offset, second) in boundaries.iter().enumerate().skip(index + 1) {
                if *second <= *first || *second >= code.len() {
                    continue;
                }
                parts.push(vec![
                    &code[..*first],
                    &code[*first..*second],
                    &code[*second..],
                ]);
                for third in boundaries.iter().skip(offset + 1) {
                    if *third <= *second || *third >= code.len() {
                        continue;
                    }
                    parts.push(vec![
                        &code[..*first],
                        &code[*first..*second],
                        &code[*second..*third],
                        &code[*third..],
                    ]);
                }
            }
        }
        for split in parts {
            if split.iter().any(|part| part.chars().count() < 2) {
                continue;
            }
            let words = self.dictionary.lookup(&split, false);
            let Some(word) = words.first() else {
                continue;
            };
            let chars: Vec<char> = word.text.chars().collect();
            if chars.len() != split.len() {
                continue;
            }
            for (syllable, ch) in split.iter().zip(chars) {
                if pairs
                    .iter()
                    .any(|(known, known_ch)| known == syllable && *known_ch == ch)
                {
                    continue;
                }
                pairs.push(((*syllable).to_owned(), ch));
            }
            // 第一个能查出词的切法就够
            break;
        }
        pairs
    }

    /// 按音节拼一个词：能拿到提示字的音节用提示字，其余用该音节最常见的单字。
    fn build_from_hints(
        &self,
        code: &str,
        best: &crate::parser::Segmentation,
    ) -> Option<Candidate> {
        let pairs = self.hint_pairs(code);
        if pairs.is_empty() {
            return None;
        }
        let syllables: Vec<&str> = best
            .syllables
            .iter()
            .map(|syllable| syllable.text.as_str())
            .collect();
        if syllables.len() < 2 {
            return None;
        }
        let mut built = String::new();
        for syllable in &syllables {
            let hinted = pairs
                .iter()
                .find(|(hinted, _)| hinted == syllable)
                .map(|(_, ch)| *ch);
            let picked = hinted.or_else(|| self.top_char(syllable))?;
            built.push(picked);
        }
        Some(Candidate {
            text: built,
            kind: crate::CandidateKind::Shortcut,
            syllables: Vec::new(),
            reading: None,
            translation: None,
            aux_code: None,
        })
    }

    /// 这个音节最常见的单字。
    fn top_char(&self, syllable: &str) -> Option<char> {
        self.dictionary
            .lookup(&[syllable], false)
            .into_iter()
            .find_map(|found| {
                let mut chars = found.text.chars();
                let first = chars.next()?;
                chars.next().is_none().then_some(first)
            })
    }
}
