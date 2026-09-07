/// 一次查询命中的词目，全部借用自词库。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match<'a> {
    /// 词。
    pub text: &'a str,

    /// 空格分隔的音节。
    pub pinyin: &'a str,

    /// 静态词频。
    pub frequency: u32,

    /// 音节数与查询模式的长度一致（而不是以查询为前缀的更长词）。
    pub exact: bool,
}

impl<'a> Match<'a> {
    pub fn syllables(&self) -> impl Iterator<Item = &'a str> {
        self.pinyin.split(' ')
    }

    pub fn syllable_count(&self) -> usize {
        self.pinyin.split(' ').count()
    }
}
