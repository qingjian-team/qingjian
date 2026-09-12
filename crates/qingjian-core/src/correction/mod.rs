//! 拼写纠错：用户敲的拼音「不像话」时（切不干净，或非末尾有简拼 / 残缺音节），
//! 试一处编辑（相邻换位、换一个字母、多一个、少一个）能不能变成每个音节都完整的拼音。
//!
//! 这里只产生**候选纠正**（变体 + 完整切分），挑哪一个由 Engine 用词库和语言模型定：
//! 纠正后至少要能凑出一个两音节以上的词，否则宁可不纠。学习方面：接受过的纠正按原输入串记选择，
//! 回车原样上屏过的串记成「不纠」，都在 Learner 里。
//!
//! 另一路是音节级的敲错变体（[`typo`]）：每个完整音节一处编辑后仍合法的写法进词图当带代价的边，
//! 由整句转换与词级排序按噪声信道挑；用户接受过的 (敲的, 要的) 音节对记进个人敲错表（Learner），以后那条边更便宜。各项代价与折扣上限收在 [`TypoCosts`] 里。

mod costs;
mod edit;
mod result;
pub mod typo;

pub use costs::TypoCosts;
pub use edit::{Edit, variants};
pub use result::Correction;
pub use typo::TypoKind;

use crate::parser::{self, Segmentation};

/// 少于这么多字母不纠：短串的一处编辑几乎总能凑出别的合法拼音，误纠比不纠更烦。
pub const MIN_LETTERS: usize = 4;

/// 多于这么多字母不纠：变体数量随长度线性涨，而且这么长多半是整句简拼。
pub const MAX_LETTERS: usize = 24;

/// 这段输入是否值得试纠错：纯小写字母、长度在范围内。
pub fn eligible(input: &str) -> bool {
    (MIN_LETTERS..=MAX_LETTERS).contains(&input.len())
        && input.bytes().all(|b| b.is_ascii_lowercase())
}

/// 切出来的拼音「不像话」：有切不动的尾巴，或非末尾的音节里有简拼 / 残缺。
/// 英文补全与拼写纠错都用这个判断。
pub fn unlikely_pinyin(best: Option<&Segmentation>, tail: &str) -> bool {
    !tail.is_empty()
        || best.is_some_and(|best| {
            let count = best.syllables.len();
            best.syllables
                .iter()
                .take(count.saturating_sub(1))
                .any(|s| !s.complete)
        })
}

/// `text` 能否切成每个音节都完整的拼音；能就返回音节最少的那种切分。
pub fn complete_segmentation(text: &str) -> Option<Segmentation> {
    parser::segment(text)
        .ok()?
        .into_iter()
        .find(|s| s.incomplete_count() == 0)
}

/// 切出来的拼音末尾是个单字母（`mingtai'n`）：可能是合法简拼，也可能是相邻两键敲反了（`mingtain` → `mingtian`）。
pub fn trailing_single_letter(best: Option<&Segmentation>) -> bool {
    best.is_some_and(|best| {
        best.syllables.len() >= 2 && best.syllables.last().is_some_and(|s| s.text.len() == 1)
    })
}

/// `input` 的全部候选纠正：一处编辑之后能完整切分的变体，顺序同 [`variants`]（换位最先）。
pub fn candidates(input: &str) -> Vec<Correction> {
    candidates_from(input, variants(input))
}

/// 只试相邻换位的候选纠正（末尾单字母那种「像话但可疑」的输入用，变体只有 n−1 个，每键都试得起）。
pub fn transposition_candidates(input: &str) -> Vec<Correction> {
    let transpositions = variants(input)
        .into_iter()
        .filter(|(edit, _)| matches!(edit, Edit::Transpose { .. }))
        .collect();
    candidates_from(input, transpositions)
}

fn candidates_from(input: &str, variants: Vec<(Edit, String)>) -> Vec<Correction> {
    if !eligible(input) {
        return Vec::new();
    }
    variants
        .into_iter()
        .filter(|(_, corrected)| parser::is_fully_segmentable(corrected))
        .filter_map(|(edit, corrected)| {
            let segmentation = complete_segmentation(&corrected)?;
            Some(Correction {
                original: input.to_owned(),
                corrected,
                edit,
                segmentation,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eligibility_bounds() {
        assert!(!eligible("nih"));
        assert!(eligible("niho"));
        assert!(!eligible("ni'hao"));
        assert!(!eligible(&"a".repeat(MAX_LETTERS + 1)));
    }

    #[test]
    fn unlikely_when_tail_or_inner_abbreviation() {
        let (segs, tail) = (parser::segment("nihao").unwrap(), "");
        assert!(!unlikely_pinyin(segs.first(), tail));
        assert!(unlikely_pinyin(segs.first(), "v"));
        let segs = parser::segment("nihooma").unwrap();
        assert!(unlikely_pinyin(segs.first(), ""));
        // 末尾没打完不算
        let segs = parser::segment("nihaom").unwrap();
        assert!(!unlikely_pinyin(segs.first(), ""));
    }

    #[test]
    fn personal_discount_is_capped() {
        let costs = TypoCosts::default();
        assert_eq!(costs.typo_cost(TypoKind::Transpose, 0), 5.0);
        assert!(costs.typo_cost(TypoKind::Transpose, 2) < 4.0);
        assert!(costs.typo_cost(TypoKind::Missing, 1000) >= 2.5 - 1e-9);
    }

    #[test]
    fn finds_full_syllable_variants() {
        let found = candidates("nihooma");
        let corrected: Vec<&str> = found.iter().map(|c| c.corrected.as_str()).collect();
        assert!(corrected.contains(&"nihaoma"));
        assert!(corrected.contains(&"nihouma"));
        assert!(found.iter().all(|c| c.segmentation.incomplete_count() == 0));
        // 换位也在其中（哪个胜出由 Engine 按词库与语言模型定）
        let found = candidates("nihoama");
        assert!(found.iter().any(
            |c| c.corrected == "nihaoma" && matches!(c.edit, Edit::Transpose { index: 3, .. })
        ));
    }
}
