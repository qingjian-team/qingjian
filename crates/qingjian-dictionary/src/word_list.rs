use std::collections::HashMap;
use std::path::Path;

use crate::error::DictionaryError;

/// 英文词表：按小写编码查原样大小写的词，也能按前缀补全。
/// 文件格式 TSV：`词\t编码[\t词频]`（编码缺省为词的小写，词频缺省 0）。
#[derive(Debug, Default)]
pub struct WordList {
    /// 小写编码 → 在 `entries` 里的下标。
    by_code: HashMap<String, usize>,

    /// (编码, 原样词, 词频)，按编码字节序排好，前缀补全二分定位。
    entries: Vec<(String, String, u32)>,
}

impl WordList {
    pub fn parse(source: &str) -> Result<Self, DictionaryError> {
        let mut by_code: HashMap<String, usize> = HashMap::new();
        let mut entries: Vec<(String, String, u32)> = Vec::new();
        for (index, raw) in source.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            let word = fields
                .next()
                .filter(|s| !s.is_empty())
                .ok_or(DictionaryError::Line {
                    line: index + 1,
                    reason: "missing word",
                })?;
            let code = fields
                .next()
                .filter(|s| !s.is_empty())
                .map(str::to_ascii_lowercase)
                .unwrap_or_else(|| word.to_ascii_lowercase());
            let frequency = fields
                .next()
                .map(|s| s.trim().parse::<u32>())
                .transpose()
                .map_err(|_| DictionaryError::Line {
                    line: index + 1,
                    reason: "frequency is not a non-negative integer",
                })?
                .unwrap_or(0);
            // 同一编码取第一个（词表按常用度排序时即最常用的写法）
            if by_code.contains_key(&code) {
                continue;
            }
            by_code.insert(code.clone(), entries.len());
            entries.push((code, word.to_owned(), frequency));
        }
        // 按编码排序后重建下标
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        for (index, (code, _, _)) in entries.iter().enumerate() {
            by_code.insert(code.clone(), index);
        }
        tracing::debug!(words = entries.len(), "英文词表加载完成");
        Ok(Self { by_code, entries })
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, DictionaryError> {
        Self::parse(&std::fs::read_to_string(path)?)
    }

    /// 输入（已小写）对应的英文词。
    pub fn get(&self, code: &str) -> Option<&str> {
        self.by_code
            .get(code)
            .map(|&index| self.entries[index].1.as_str())
    }

    /// 以 `prefix` 开头（不含正好相等的）的词里词频最高的 `limit` 个，按词频降序。
    pub fn complete(&self, prefix: &str, limit: usize) -> Vec<&str> {
        if prefix.is_empty() || limit == 0 {
            return Vec::new();
        }
        let start = self
            .entries
            .partition_point(|(code, _, _)| code.as_str() < prefix);
        let mut hits: Vec<&(String, String, u32)> = self.entries[start..]
            .iter()
            .take_while(|(code, _, _)| code.starts_with(prefix))
            .filter(|(code, _, _)| code != prefix)
            .collect();
        hits.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
        hits.into_iter()
            .take(limit)
            .map(|(_, word, _)| word.as_str())
            .collect()
    }

    /// 全部词目：(小写编码, 原样词, 词频)，按编码字节序。
    pub fn entries(&self) -> impl Iterator<Item = (&str, &str, u32)> {
        self.entries
            .iter()
            .map(|(code, word, frequency)| (code.as_str(), word.as_str(), *frequency))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_up_by_lowercase_code() {
        let list = WordList::parse("GitHub\tgithub\nhello\thello\niPhone\n").unwrap();
        assert_eq!(list.get("github"), Some("GitHub"));
        assert_eq!(list.get("iphone"), Some("iPhone"));
        assert_eq!(list.get("hello"), Some("hello"));
        assert_eq!(list.get("nope"), None);
    }

    #[test]
    fn completes_prefixes_by_frequency() {
        let list = WordList::parse(
            "compass\tcompass\t300\ncompany\tcompany\t900\ncompare\tcompare\t500\ncom\tcom\t100\ncomma\tcomma\n",
        )
        .unwrap();
        assert_eq!(list.complete("comp", 2), ["company", "compare"]);
        assert_eq!(
            list.complete("com", 10),
            ["company", "compare", "compass", "comma"]
        );
        assert!(list.complete("zzz", 3).is_empty());
        assert!(list.complete("", 3).is_empty());
    }
}
