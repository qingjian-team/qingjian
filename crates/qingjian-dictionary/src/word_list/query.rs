//! 词表 / 音标表的查询与遍历。

use super::WordList;

impl WordList {
    /// 输入（已小写）对应的英文词。
    pub fn get(&self, code: &str) -> Option<&str> {
        self.find(code).map(|entry| self.word(entry))
    }

    /// 音标表用：输入（已小写）对应的音标，形如 `/ˈkɑrɡoʊ/`。
    ///
    /// 查不到返回 `None`（CMUdict 没收的派生词形就是这样），调用方据此不显示音标。
    pub fn pronunciation(&self, code: &str) -> Option<&str> {
        self.find(code).map(|entry| self.word(entry))
    }

    /// 输入（已小写）对应的词频；词表里没有为 `None`。
    pub fn frequency(&self, code: &str) -> Option<u32> {
        self.find(code).map(|entry| entry.frequency)
    }

    /// 以 `prefix` 开头（不含正好相等的）的词里词频最高的 `limit` 个，按词频降序。
    pub fn complete(&self, prefix: &str, limit: usize) -> Vec<&str> {
        if prefix.is_empty() || limit == 0 {
            return Vec::new();
        }
        let start = self
            .entries
            .partition_point(|entry| self.code(entry) < prefix);
        let mut hits: Vec<&super::Entry> = self.entries[start..]
            .iter()
            .take_while(|entry| self.code(entry).starts_with(prefix))
            .filter(|entry| self.code(entry) != prefix)
            .collect();
        hits.sort_by(|a, b| {
            b.frequency
                .cmp(&a.frequency)
                .then_with(|| self.code(a).cmp(self.code(b)))
        });
        hits.into_iter()
            .take(limit)
            .map(|entry| self.word(entry))
            .collect()
    }

    /// 全部词目：(小写编码, 原样词, 词频)，按编码字节序。
    pub fn entries(&self) -> impl Iterator<Item = (&str, &str, u32)> {
        self.entries
            .iter()
            .map(|entry| (self.code(entry), self.word(entry), entry.frequency))
    }
}
