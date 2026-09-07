//! 挖词结果的过滤：连续单字段里数出来的子串大多不是词（「的一」「了一个」），按两条规则筛。
//!
//! 1. 虚词规则：以助词 / 副词类虚字开头的不要（「的话」「了解」这种正常词早在词库里，落到这里的都是碎片）；
//!    全由虚字组成的不要；同一个字重复的不要（「哈哈哈」这类叠词另算）。
//! 2. 相邻字对的点互信息（PMI）：词里每对相邻字 `ab` 的 `ln(p(ab) / (p(a) p(b)))` 取最小值，低于阈值的说明只是碰巧挨着。
//!    `p(a)` 用语言模型一元表里单字的次数，`p(ab)` 用挖词本身数出来的两字段次数（同一份语料）；
//!    某对相邻字没被数到（次数不够 `min_count`）直接不要。阈值 3 是 2026-09-05 在 8.3 万候选上人工抽样定的：
//!    2–3 之间开始混进「回家吧」「看到了」一类。

use std::collections::HashMap;
use std::path::Path;

use crate::error::ConvertError;

/// 开头不能是这些字（助词、语气词、常见副词、介词）。
const LEADING_FUNCTION_CHARS: &str = "的了着得地吗吧呢啊呀哦嗯也都就还才又再很太被把";

/// 全由这些字组成的不算词。
const FUNCTION_CHARS: &str = "的了着得地吗吧呢啊呀哦嗯也都就还才又再很太被把是在有和这那我你他她它们不没要会能可以与及或而且但";

/// 过滤器：单字次数 + 阈值。
pub struct OovFilter {
    /// 单字 → 语料里的次数（从一元表读，只留单字）。
    unigram: HashMap<char, u64>,

    /// 一元表的总次数（不含句首标记）。
    total: f64,

    /// 相邻字对 PMI 的下限。
    min_pmi: f64,
}

impl OovFilter {
    /// 读语言模型一元表（`词\t次数`，`<s>` 是句首标记）。
    pub fn from_unigram_file(path: &Path, min_pmi: f64) -> Result<Self, ConvertError> {
        Ok(Self::from_unigram_text(
            &std::fs::read_to_string(path)?,
            min_pmi,
        ))
    }

    pub fn from_unigram_text(text: &str, min_pmi: f64) -> Self {
        let mut unigram = HashMap::new();
        let mut total = 0.0;
        for line in text.lines() {
            if line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            let (Some(word), Some(count)) = (fields.next(), fields.next()) else {
                continue;
            };
            let Ok(count) = count.parse::<u64>() else {
                continue;
            };
            if word == "<s>" {
                continue;
            }
            total += count as f64;
            let mut chars = word.chars();
            if let (Some(c), None) = (chars.next(), chars.next()) {
                unigram.insert(c, count);
            }
        }
        Self {
            unigram,
            total,
            min_pmi,
        }
    }

    /// 虚词规则：过不了的直接不要，不用算 PMI。
    pub fn passes_function_rules(word: &str) -> bool {
        let mut chars = word.chars();
        let Some(first) = chars.next() else {
            return false;
        };
        if LEADING_FUNCTION_CHARS.contains(first) {
            return false;
        }
        if word.chars().all(|c| FUNCTION_CHARS.contains(c)) {
            return false;
        }
        if word.chars().all(|c| c == first) {
            return false;
        }
        true
    }

    /// 词里相邻字对 PMI 的最小值；某对没在 `pair_counts` 里为 `None`。
    pub fn min_pair_pmi(&self, word: &str, pair_counts: &HashMap<String, u32>) -> Option<f64> {
        let chars: Vec<char> = word.chars().collect();
        let mut lowest: Option<f64> = None;
        for pair in chars.windows(2) {
            let key: String = pair.iter().collect();
            let joint = f64::from(*pair_counts.get(&key)?);
            let a = *self.unigram.get(&pair[0]).unwrap_or(&1) as f64;
            let b = *self.unigram.get(&pair[1]).unwrap_or(&1) as f64;
            let pmi = ((joint / self.total) / ((a / self.total) * (b / self.total))).ln();
            lowest = Some(lowest.map_or(pmi, |l: f64| l.min(pmi)));
        }
        lowest
    }

    /// 是否留下：虚词规则 + PMI 阈值。
    pub fn keeps(&self, word: &str, pair_counts: &HashMap<String, u32>) -> bool {
        Self::passes_function_rules(word)
            && self
                .min_pair_pmi(word, pair_counts)
                .is_some_and(|pmi| pmi >= self.min_pmi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_rules_reject_fragments() {
        assert!(!OovFilter::passes_function_rules("的话"));
        assert!(!OovFilter::passes_function_rules("了一个"));
        assert!(!OovFilter::passes_function_rules("我们的"));
        assert!(!OovFilter::passes_function_rules("哈哈哈"));
        assert!(OovFilter::passes_function_rules("回家"));
        assert!(OovFilter::passes_function_rules("有点"));
    }

    #[test]
    fn pmi_keeps_words_whose_characters_cling_together() {
        // 100 万字的语料：回 1 万、家 2 万、吧 5 万；回家 8 千，家吧 60
        let filter = OovFilter::from_unigram_text(
            "<s>\t999\n回\t10000\n家\t20000\n吧\t50000\n其他\t920000\n",
            3.0,
        );
        let mut pairs = HashMap::new();
        pairs.insert("回家".to_owned(), 8000);
        pairs.insert("家吧".to_owned(), 60);
        let pmi = filter.min_pair_pmi("回家", &pairs).unwrap();
        assert!(pmi > 3.0, "{pmi}");
        assert!(filter.keeps("回家", &pairs));
        // 回家吧 取最小的一对（家吧），碰巧挨着，PMI 低
        assert!(filter.min_pair_pmi("回家吧", &pairs).unwrap() < 1.0);
        assert!(!filter.keeps("回家吧", &pairs));
        // 没数到的字对直接不要
        assert_eq!(filter.min_pair_pmi("回去", &pairs), None);
        assert!(!filter.keeps("回去", &pairs));
    }
}
