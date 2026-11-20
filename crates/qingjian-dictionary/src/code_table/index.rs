//! 码表查询：按编码前缀选候选。

use crate::matching::Match;
use crate::tones::Tones;

use super::CodeTable;

impl CodeTable {
    /// 编码正好等于输入，或输入是它的前缀的词，最多 `limit` 条。
    ///
    /// 编码与输入相等（这个词打全了）的排在最前；单字母精确码按码表行序，其余按词频降序。
    /// `limit` 之后的不返回。
    /// 单字母输入（`g`，一级简码）的前缀区间有上万条，所以先按 `limit` 线性选出前面一段再排序，
    /// 不是把整个区间排完再截断——`lookup` 在每次按键的路径上。
    pub fn lookup(&self, code: &str, limit: usize) -> Vec<Match<'_>> {
        if code.is_empty() || limit == 0 {
            return Vec::new();
        }
        let order = |a: &&super::Entry, b: &&super::Entry| {
            (b.code == code)
                .cmp(&(a.code == code))
                .then_with(|| {
                    if code.len() == 1 && a.code == code && b.code == code {
                        a.order.cmp(&b.order)
                    } else {
                        b.frequency.cmp(&a.frequency)
                    }
                })
                .then_with(|| a.code.cmp(&b.code))
                .then_with(|| a.text.cmp(&b.text))
        };
        let start = self.entries.partition_point(|e| e.code.as_str() < code);
        let mut hits: Vec<&super::Entry> = self.entries[start..]
            .iter()
            .take_while(|e| e.code.starts_with(code))
            .collect();
        if hits.len() > limit {
            hits.select_nth_unstable_by(limit, order);
            hits.truncate(limit);
        }
        hits.sort_by(order);
        hits.into_iter()
            .map(|entry| Match {
                text: entry.text.as_str(),
                pinyin: entry.code.as_str(),
                frequency: entry.frequency,
                reading_total: 0,
                tones: Tones::default(),
                secondary: false,
                exact: entry.code == code,
            })
            .collect()
    }
}
