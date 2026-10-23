use crate::tones::Tones;

/// 一次查询命中的词目，全部借用自词库。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match<'a> {
    /// 词。
    pub text: &'a str,

    /// 空格分隔的音节。
    pub pinyin: &'a str,

    /// 静态词频。
    pub frequency: u32,

    /// 这个字全部读音的词频之和；不是多音字（或不是单字）时是 0。见 [`Self::reading_share`]。
    pub reading_total: u32,

    /// 逐音节声调；词库没标的是不知道。
    pub tones: Tones<'a>,

    /// 来自补充词库（[`crate::Dictionary::set_secondary`]）：精确命中不压过主词库的纠错，整句里多扣一点分。
    pub secondary: bool,

    /// 音节数与查询模式的长度一致（而不是以查询为前缀的更长词）。
    pub exact: bool,
}

impl<'a> Match<'a> {
    pub fn syllables(&self) -> impl Iterator<Item = &'a str> {
        self.pinyin.split(' ')
    }

    /// 这个读音占这个字全部读音词频的比例（`区 ou` 约 0.05）；不是多音字时是 1。
    /// 语言模型只按字形给分，打分时乘上它，少见读音才不会拿到常见读音的分。
    pub fn reading_share(&self) -> f64 {
        if self.reading_total == 0 {
            return 1.0;
        }
        ((f64::from(self.frequency) + 1.0) / (f64::from(self.reading_total) + 1.0)).min(1.0)
    }

    pub fn syllable_count(&self) -> usize {
        self.pinyin.split(' ').count()
    }
}
