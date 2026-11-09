//! 整句没出对时，正确答案丢在哪一步（[`sentence::Loss`]）：给 CLI 评测归因用，输入路径不调。

use crate::engine::choice::char_emission;
use crate::engine::{Learner, reads_as_typed, segment_longest_prefix};
use crate::{Engine, sentence};

/// 换切分时最多试几种。
const SEGMENTATIONS_TRIED: usize = 16;

impl Engine {
    /// 当前输入（全拼，查过一次之后）的整句为什么不是 `gold`；整句就是 `gold` 或拼音切不动时为 `None`。
    /// 只看不带拼写纠错的整段拼音，与评测集的输入一致。
    pub fn diagnose_sentence(&self, gold: &str, top: Option<&str>) -> Option<sentence::Loss> {
        let keys = self.composition.scope();
        let (segmentations, _) = segment_longest_prefix(keys).ok()?;
        let best = self.sentence_segmentation(&segmentations, true)?;
        let patterns = best.patterns();
        // 与整句首选同一个规则：原样读得通就不加敲错边
        let typos = !reads_as_typed(best);
        let path = self
            .sentence_paths(&patterns, typos, false, 1)
            .into_iter()
            .next()?;
        if path.text == gold {
            return (top != Some(gold)).then_some(sentence::Loss::Assembly);
        }
        let loss = self.diagnose_patterns(&patterns, gold, &path.text, true, typos);
        if !matches!(loss, sentence::Loss::Unreachable { .. }) {
            return Some(loss);
        }
        let other = segmentations
            .iter()
            .take(SEGMENTATIONS_TRIED)
            .filter(|s| s.syllables != best.syllables)
            .find(|s| {
                !matches!(
                    self.diagnose_patterns(&s.patterns(), gold, "", false, true),
                    sentence::Loss::Unreachable { .. }
                )
            });
        Some(match other {
            Some(s) => sentence::Loss::Segmentation {
                syllables: s.syllables.iter().map(|x| x.text.to_owned()).collect(),
            },
            None => loss,
        })
    }

    /// 在 `patterns` 这种切分的词图上对齐 `gold`；`emit` 为真时带上知意的出字分（只有整句用的那种切分有表）。
    fn diagnose_patterns(
        &self,
        patterns: &[qingjian_dictionary::SyllablePattern<'_>],
        gold: &str,
        best: &str,
        emit: bool,
        typos: bool,
    ) -> sentence::Loss {
        let dictionaries = self.all_dictionaries();
        let expanded = self.expand_positions(patterns, typos);
        let table = if emit {
            self.char_table(patterns)
        } else {
            None
        };
        let emission = |start: usize, text: &str, syllables: &[String]| {
            table.as_deref().map_or(0.0, |table| {
                char_emission(table, patterns, start, text, syllables)
            })
        };
        // 格子名额与真搜索一致（回放调参改了 `search_limits` 时诊断也跟着）
        let input = sentence::SearchInput::new(
            &dictionaries,
            &*self.language_model,
            self.personal(),
            |text| self.learner.weight(text),
            |index, syllable| expanded.cost(index, syllable),
        )
        .with_emission(&emission)
        .with_limits(self.search_limits);
        sentence::diagnose(&input, &expanded.positions(), gold, best)
    }
}

impl Engine {
    /// `gold` 在词图前 `k` 条整句路径里排第几（从 0 起；不在为 `None`）：量词图多交出几条整句给选词模型值不值。
    pub fn sentence_path_rank(&self, gold: &str, k: usize) -> Option<usize> {
        let keys = self.composition.scope();
        let (segmentations, _) = segment_longest_prefix(keys).ok()?;
        let best = self.sentence_segmentation(&segmentations, true)?;
        self.sentence_paths(&best.patterns(), !reads_as_typed(best), false, k)
            .iter()
            .position(|p| p.text == gold)
    }
}
