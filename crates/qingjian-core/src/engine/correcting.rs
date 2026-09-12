//! 整段拼写纠错：找一处编辑的纠正并按个人敲错表打折。

use super::*;

impl Engine {
    /// 这段作用域生效的拼写纠正（带缓存）：拼音不像话、用户没对它回车原样上屏过、
    /// 且一处编辑后能凑出至少一个两音节词时，取整句转换得分最高的那个纠正。
    pub(super) fn active_correction(&self, scope: &str) -> Option<Correction> {
        // 双拼敲错一个键换掉的是整个声母 / 韵母，全拼那套「一处编辑」的纠错模型不适用
        if self.shuangpin.is_some() {
            return None;
        }
        if let Some((cached_scope, cached)) = self.correction_cache.borrow().as_ref()
            && cached_scope == scope
        {
            return cached.clone();
        }
        let found = self.find_correction(scope);
        *self.correction_cache.borrow_mut() = Some((scope.to_owned(), found.clone()));
        found
    }

    pub(super) fn find_correction(&self, scope: &str) -> Option<Correction> {
        if !correction::eligible(scope) || self.learner.raw_count(scope) > 0 {
            return None;
        }
        // 整段就是个英文词（hello）：用户多半在打英文，别把它「纠」成 喝了哦
        if self
            .english
            .as_ref()
            .is_some_and(|english| english.get(scope).is_some())
        {
            return None;
        }
        let (segmentations, tail) = segment_longest_prefix(scope).ok()?;
        // 不像话的拼音试全部一处编辑；末尾单字母的只试相邻换位（`mingtain` → `mingtian`），其余合法拼音不碰
        let candidates = if correction::unlikely_pinyin(segmentations.first(), tail) {
            correction::candidates(scope)
        } else if correction::trailing_single_letter(segmentations.first()) {
            correction::transposition_candidates(scope)
        } else {
            return None;
        };
        // 噪声信道：原串按原样能转出的整句得分 vs 纠正后的整句得分扣掉一次编辑的代价，后者高才纠。
        // 原串切不干净（有尾巴）就没有原样得分，任何能转出整句的纠正都胜出
        let raw_score = if tail.is_empty() {
            segmentations
                .first()
                .and_then(|best| self.convert_sentence(&best.patterns(), true))
                .filter(|conversion| !conversion.has_placeholder())
                .map(|conversion| conversion.score)
        } else {
            None
        };
        // 每个纠正扣一次编辑的代价，接受过同样的 (敲的, 要的) 音节对越多次扣得越少（个人敲错表）
        let mut best: Option<(f64, Correction)> = None;
        for candidate in candidates {
            // 「删掉刚敲的最后一个字母」不算纠正：用户可能还没敲完，尾巴留着等下一键
            if matches!(candidate.edit, correction::Edit::Delete { index, .. } if index + 1 == scope.len())
            {
                continue;
            }
            // 纠正后的拼音上不再猜第二处敲错：变体本来就是一处编辑之外的读法，再叠一层既慢又几乎不会赢
            let Some(conversion) = self.convert_sentence(&candidate.segmentation.patterns(), false)
            else {
                continue;
            };
            if conversion.has_placeholder() {
                continue;
            }
            let accepted = candidate
                .typo_pair(candidate.corrected.len())
                .map_or(0, |(typed, intended)| {
                    self.learner.typo_count(&typed, &intended)
                });
            let score = conversion.score - self.typo_costs.correction_cost(accepted);
            if best.as_ref().is_none_or(|(best, _)| score > *best) {
                best = Some((score, candidate));
            }
        }
        let (score, found) = best?;
        if raw_score.is_some_and(|raw| score <= raw) {
            tracing::debug!(
                original = %found.original,
                corrected = %found.corrected,
                score,
                raw = raw_score,
                "原样已经说得通，不纠"
            );
            return None;
        }
        tracing::debug!(original = %found.original, corrected = %found.corrected, score, raw = raw_score, "拼写纠正");
        Some(found)
    }
}
