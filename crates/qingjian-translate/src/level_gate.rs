use qingjian_core::Translation;

use crate::LevelTable;

/// 随包释义表的等级门槛（配置 `[general] translation_min_level`）：等级低于阈值的译词不显示，
/// the / is / important 这类过于简单的词没有学习价值。个人释义表不走门槛（个人表优先，用户手加的、
/// 释义兜底写的都放行）；等级表里查不到的词（专名、技术词）也放行。
pub struct LevelGate {
    /// 词汇等级表（CEFR / JLPT，等级从易到难）。
    levels: LevelTable,

    /// 门槛：等级序号小于它的释义被滤掉（`b1` 在 en 的等级表里是 2）。
    min_rank: usize,
}

impl LevelGate {
    /// 按等级名解析门槛（`b1` / `n3` …，大小写不敏感）。名字不在该语言的等级表里时返回
    /// `None`，门槛不生效——`off`、留空与写错的值都走这里，装配处照常记一条日志。
    pub fn new(levels: LevelTable, min_level: &str) -> Option<Self> {
        let min_rank = levels
            .levels()
            .iter()
            .position(|level| level.eq_ignore_ascii_case(min_level))?;
        Some(Self { levels, min_rank })
    }

    /// 滤掉等级不够的释义，返回还剩没剩：全被滤掉就等于查不到这个词。
    pub fn passes(&self, translation: &mut Translation) -> bool {
        translation
            .senses_mut()
            .retain(|sense| self.rank(&sense.text).is_none_or(|rank| rank >= self.min_rank));
        !translation.senses().is_empty()
    }

    /// 这条译文里最难的等级（越大越难），整句候选挑「句中最难的词」用。
    pub fn difficulty(&self, translation: &Translation) -> usize {
        translation
            .senses()
            .iter()
            .map(|sense| self.rank(&sense.text).unwrap_or(0))
            .max()
            .unwrap_or(0)
    }

    /// 等级表的键是小写（CEFR 词表），先原样查再转小写查，兜住 `I` 这类大写译词。
    fn rank(&self, word: &str) -> Option<usize> {
        let word = word.trim();
        self.levels
            .rank(word)
            .or_else(|| self.levels.rank(&word.to_lowercase()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qingjian_core::{Language, Sense, Translation};

    const LEVELS: &str = "# levels\tA1\tA2\tB1\nthe\tA1\nis\tA1\nimportant\tA2\nsignificant\tB1\n";

    fn translation(senses: &[&str]) -> Translation {
        Translation::new(
            Language::English,
            senses
                .iter()
                .map(|text| Sense {
                    part_of_speech: None,
                    text: (*text).to_owned(),
                    reading: None,
                    fresh: false,
                })
                .collect(),
        )
    }

    #[test]
    fn hides_levels_below_the_gate() {
        let gate = LevelGate::new(LevelTable::parse(LEVELS).unwrap(), "b1").unwrap();
        let mut translation = translation(&["the", "significant"]);
        assert!(gate.passes(&mut translation));
        assert_eq!(translation.senses()[0].text, "significant");
    }

    #[test]
    fn unknown_words_pass_the_gate() {
        let gate = LevelGate::new(LevelTable::parse(LEVELS).unwrap(), "b1").unwrap();
        let mut translation = translation(&["qingjian"]);
        assert!(gate.passes(&mut translation));
    }

    #[test]
    fn all_hidden_means_not_found() {
        let gate = LevelGate::new(LevelTable::parse(LEVELS).unwrap(), "b1").unwrap();
        let mut translation = translation(&["the", "is"]);
        assert!(!gate.passes(&mut translation));
    }

    #[test]
    fn unknown_level_name_disables_the_gate() {
        assert!(LevelGate::new(LevelTable::parse(LEVELS).unwrap(), "z9").is_none());
    }

    #[test]
    fn difficulty_is_the_hardest_sense() {
        let gate = LevelGate::new(LevelTable::parse(LEVELS).unwrap(), "a1").unwrap();
        assert_eq!(gate.difficulty(&translation(&["the", "significant"])), 2);
    }
}
