//! 辅码码表的推导查询与 [`AuxCodeLookup`] trait 实现。

use std::borrow::Cow;

use crate::aux_code_table::lookup::AuxCodeLookup;

use super::AuxCodeTable;

impl AuxCodeTable {
    /// 表里没有 `word` 时按词码规则推导；只有一个字或没写规则时为空。
    fn derived(&self, word: &str) -> Vec<String> {
        if self.word_rules.is_empty() || word.chars().nth(1).is_none() {
            return Vec::new();
        }
        self.word_rules
            .derive(word, |c| AuxCodeTable::codes_of(self, c).collect())
    }
}

impl AuxCodeLookup for AuxCodeTable {
    fn code_with_prefix<'a>(&'a self, word: &str, prefix: &str) -> Option<Cow<'a, str>> {
        // inherent 的 codes_of 是迭代器版本，这里要用它（别递归回 trait）
        let mut listed = AuxCodeTable::codes_of(self, word).peekable();
        if listed.peek().is_some() {
            return listed
                .find(|code| code.starts_with(prefix))
                .map(Cow::Borrowed);
        }
        self.derived(word)
            .into_iter()
            .find(|code| code.starts_with(prefix))
            .map(Cow::Owned)
    }

    fn codes_of<'a>(&'a self, word: &str) -> Vec<Cow<'a, str>> {
        let listed: Vec<Cow<'a, str>> = AuxCodeTable::codes_of(self, word)
            .map(Cow::Borrowed)
            .collect();
        if !listed.is_empty() {
            return listed;
        }
        self.derived(word).into_iter().map(Cow::Owned).collect()
    }
}
