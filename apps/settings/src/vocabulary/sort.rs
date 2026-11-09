//! 生词本的排序。

use std::cmp::Ordering;

use qingjian_learning::VocabularyEntry;

use crate::i18n::tr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sort {
    /// 最近见到的在前。
    #[default]
    Recent,
    /// 见得多的在前。
    Frequent,
    /// 按译词的字母。
    Alphabetical,
}

impl Sort {
    pub const ALL: [Self; 3] = [Self::Recent, Self::Frequent, Self::Alphabetical];

    pub fn label(self) -> &'static str {
        match self {
            Self::Recent => tr("vocabulary.sort_recent"),
            Self::Frequent => tr("vocabulary.sort_frequent"),
            Self::Alphabetical => tr("vocabulary.sort_alphabetical"),
        }
    }

    pub fn compare(
        self,
        (a_word, a): (&str, &VocabularyEntry),
        (b_word, b): (&str, &VocabularyEntry),
    ) -> Ordering {
        let by_word = || a_word.to_lowercase().cmp(&b_word.to_lowercase());
        match self {
            Self::Recent => b.last.cmp(&a.last).then_with(|| b.first.cmp(&a.first)),
            Self::Frequent => (b.seen + b.committed).cmp(&(a.seen + a.committed)),
            Self::Alphabetical => Ordering::Equal,
        }
        .then_with(by_word)
    }
}
