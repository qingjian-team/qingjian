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
        // 形码敲的是字根编码，不是拼音；混输下码长以内可能是编码，超过码长才只剩拼音
        if let Some(code) = &self.code
            && (!self.phonetic || scope.len() <= code.code_length)
        {
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
        // 原样前面的音节个个完整、只有末尾停着落单声母（词图按简拼放行的那种，见 #619）时，
        // 纠正后末尾仍残缺的变体不参与：`henhaobook` 在 `book` 内部换位凑出 `hen hao o bo k…`，
        // 两种读法都通顺，只是用字频投机赢过原样，会把英文尾 `book` 整条挤掉（很好book → 很好哦伯克）。
        // 用户多半下一键就补韵母，残尾对残尾的比较没有意义。末位是完整音节（`nihoa` → `nihao`）
        // 或前面本来就有残缺音节（`mignt` 中间夹着 g，换位后头部从不通变通顺）的都不受影响
        let clean_head_with_lone_initial = segmentations.first().is_some_and(|best| {
            let n = best.syllables.len();
            n >= 2
                && best.syllables[..n - 1].iter().all(|s| s.complete)
                && best.last_is_partial()
                && best.syllables[n - 1].text.len() == 1
                && parser::INITIALS.contains(&best.syllables[n - 1].text.as_str())
        });
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
        // 每个纠正扣一次编辑的代价，接受过同样的 (敲的, 要的) 音节对越多次扣得越少（个人敲错表）。
        // 相邻换位的折扣只在纠正之间比较时用（`mignti` 换位成 `mingti` 胜过换字母成 `mianti`），
        // 与原样比时不打这个折：要不要纠的门槛不因编辑类型而降（`zhongwne` 原样说得通就不纠成 中文）
        let mut best: Option<(f64, f64, Correction)> = None;
        for candidate in candidates {
            // 「删掉刚敲的最后一个字母」不算纠正：用户可能还没敲完，尾巴留着等下一键
            if matches!(candidate.edit, correction::Edit::Delete { index, .. } if index + 1 == scope.len())
            {
                continue;
            }
            // 改完末尾仍是没敲完的声母：原样本来通顺、只欠末键，这种变体只是字频投机，不纠
            if clean_head_with_lone_initial && candidate.segmentation.incomplete_count() > 0 {
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
            let transpose = matches!(candidate.edit, correction::Edit::Transpose { .. });
            let score = conversion.score - self.typo_costs.correction_cost(transpose, accepted);
            let undiscounted = conversion.score - self.typo_costs.correction_cost(false, accepted);
            if best.as_ref().is_none_or(|(best, _, _)| score > *best) {
                best = Some((score, undiscounted, candidate));
            }
        }
        let (_, score, found) = best?;
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
