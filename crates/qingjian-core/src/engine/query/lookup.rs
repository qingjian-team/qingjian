//! 词级查找：每个位置展开成多种写法，再按这些写法查主词库与用户词。

use qingjian_dictionary::Match;

use crate::correction::typo;
use crate::engine::Learner;
use crate::fuzzy::Expanded;
use crate::{Engine, Segmentation, correction, sentence};

impl Engine {
    /// 每个位置的写法：敲的原样、模糊音，再加音节级敲错变体（`correction::typo`）当带代价的边，
    /// 代价按类别定、按个人敲错表打折。太短的输入（不到 [`correction::MIN_LETTERS`]）、双拼、非末尾带简拼的切分不加敲错变体：
    /// 短串一处编辑几乎总能凑出别的词，双拼敲错一键换掉的是整个声母 / 韵母。不完整的位置（简拼、前缀）本来就按前缀查，不加。
    pub(in crate::engine) fn expand_positions(
        &self,
        patterns: &[qingjian_dictionary::SyllablePattern<'_>],
        typos: bool,
    ) -> Expanded {
        let mut expanded = self.fuzzy.expand(patterns);
        if !typos || !self.spelling_correction {
            return expanded;
        }
        let letters: usize = patterns.iter().map(|p| p.text.len()).sum();
        // 非末尾有简拼 / 残缺音节的切分（`kai f a`）本来就不是用户敲的原话，不在它上面再猜敲错
        let inner_abbreviated = patterns
            .iter()
            .take(patterns.len().saturating_sub(1))
            .any(|p| !p.complete);
        if self.shuangpin.is_some() || letters < correction::MIN_LETTERS || inner_abbreviated {
            return expanded;
        }
        for (index, pattern) in patterns.iter().enumerate() {
            if !pattern.complete {
                continue;
            }
            for (text, kind) in typo::variants(pattern.text) {
                let accepted = self.learner.typo_count(pattern.text, text);
                expanded.push_alternative(index, text, self.typo_costs.typo_cost(*kind, accepted));
            }
        }
        expanded
    }

    /// 本地整句转换把最优切分转成的汉字，给云端当参考（问字模式里就是问题的汉字形式）；转不出或有占位音节为空。
    pub(in crate::engine) fn local_guess(&self, segmentations: &[Segmentation]) -> String {
        segmentations
            .first()
            .and_then(|best| self.convert_sentence(&best.patterns(), true))
            .filter(|conversion| !conversion.has_placeholder())
            .map(|conversion| conversion.text)
            .unwrap_or_default()
    }

    /// 词级排序里一个候选的上下文得分：语言模型（不认识就按词频兜底）与个人 n-gram 插值，再乘读音占比。
    pub(in crate::engine) fn word_log_prob(&self, hit: &Match<'_>, log_total: f64) -> f64 {
        sentence::transition_log_prob(
            &*self.language_model,
            self.personal(),
            self.chain.context(),
            hit.text,
            sentence::fallback_log_prob(hit.frequency, hit.secondary, log_total),
        ) - sentence::reading_cost(hit)
    }

    /// 主词库与用户词一起查（每个位置多种写法）。用户词是用户自己选过的（云联想接受的词等），排序上靠 weight 自然靠前。
    /// 补充词库只要音节数正好对上的：它的长词当前缀补全会占满第一页（`anzhao` 全是 按照规定 / 按照相关）。
    pub(in crate::engine) fn lookup_all(
        &self,
        positions: &[Vec<qingjian_dictionary::SyllablePattern<'_>>],
    ) -> Vec<Match<'_>> {
        let mut hits = self.dictionary.lookup_pattern_alt(positions);
        for dictionary in self.all_dictionaries().into_iter().skip(1) {
            if dictionary.is_secondary() {
                hits.extend(dictionary.lookup_exact_alt(positions));
            } else {
                hits.extend(dictionary.lookup_pattern_alt(positions));
            }
        }
        hits
    }

    /// `text` 按这串音节在补充词库以外的词库里（主词库、附加词库、用户词）。
    pub(in crate::engine) fn in_primary_dictionary(
        &self,
        text: &str,
        syllables: &[String],
    ) -> bool {
        let patterns: Vec<_> = syllables
            .iter()
            .map(|s| qingjian_dictionary::SyllablePattern::complete(s))
            .collect();
        self.all_dictionaries()
            .into_iter()
            .filter(|d| !d.is_secondary())
            .any(|d| d.lookup_exact(&patterns).iter().any(|m| m.text == text))
    }

    /// 只要音节数正好等于位置数的词，主词库与用户词一起查。
    pub(in crate::engine) fn lookup_exact_all(
        &self,
        positions: &[Vec<qingjian_dictionary::SyllablePattern<'_>>],
    ) -> Vec<Match<'_>> {
        let mut hits = self.dictionary.lookup_exact_alt(positions);
        for dictionary in self.all_dictionaries().into_iter().skip(1) {
            hits.extend(dictionary.lookup_exact_alt(positions));
        }
        hits
    }
}
