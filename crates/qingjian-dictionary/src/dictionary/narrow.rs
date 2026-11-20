//! 区间收窄引擎：二分定位前缀区间，递归按音节位置筛键；小区间退回逐键比对。

use std::ops::Range;

use crate::matching::Match;
use crate::pattern::{SyllablePattern, canonical_syllable};

use super::Dictionary;

impl Dictionary {
    /// 键按字节序排好，所以「以某段前缀开头的键」总是一段连续区间：逐个音节位置用二分收窄区间，
    /// 完整音节直接定位到 `前缀 + 音节 + 空格`，简拼位置按区间里出现的不同音节跳着扫（每个音节只看一条键就能算出它的子区间）。
    /// 代价只和匹配到的音节组合数成正比，与首音节下有多少键无关；一个位置的多种写法逐个走，代价相加而不相乘。
    pub(super) fn narrow<'a, 'p>(
        &'a self,
        positions: &[impl AsRef<[SyllablePattern<'p>]>],
        depth: usize,
        prefix: &mut String,
        range: Range<usize>,
        exact_only: bool,
        out: &mut Vec<Match<'a>>,
    ) {
        let last = depth + 1 == positions.len();
        let base_len = prefix.len();
        for current in positions[depth].as_ref() {
            prefix.truncate(base_len);
            prefix.push_str(canonical_syllable(current.text));
            let sub = self.prefix_range(prefix, range.clone());
            if sub.is_empty() {
                continue;
            }
            // 区间已经很小：直接逐键比对剩下的模式，比继续二分 + 递归便宜（全简拼时小块极多）。
            // 当前位置只按这一种写法比（`an` 的区间里也有 `ang…`，那些留给 `ang` 那一轮，否则重复）
            if sub.len() <= super::LINEAR_SCAN_LIMIT {
                self.scan_range(
                    current,
                    &positions[depth + 1..],
                    base_len,
                    sub,
                    exact_only,
                    out,
                );
                continue;
            }
            match (current.complete, last) {
                // 完整且是最后一个：正好这个键的词精确命中，`键 + 空格` 开头的更长词也收
                (true, true) => {
                    if let Some(entry) = self.index[sub.clone()]
                        .first()
                        .filter(|entry| self.key(entry).len() == prefix.len())
                    {
                        self.push_matches(entry, true, out);
                    }
                    if !exact_only {
                        prefix.push(' ');
                        for entry in &self.index[self.prefix_range(prefix, sub)] {
                            self.push_matches(entry, false, out);
                        }
                    }
                }
                (true, false) => {
                    prefix.push(' ');
                    let deeper = self.prefix_range(prefix, sub);
                    self.narrow(positions, depth + 1, prefix, deeper, exact_only, out);
                }
                // 前缀且是最后一个、全都要：区间里全是命中，音节数正好等于位置数的才精确
                (false, true) if !exact_only => {
                    for entry in &self.index[sub] {
                        let exact = !self.key(entry)[base_len..].contains(' ');
                        self.push_matches(entry, exact, out);
                    }
                }
                // 前缀且后面还有（或只要精确的）：按区间里实际出现的音节逐块进入，
                // 每块第一条键若正好到此为止就是该音节的精确词
                _ => {
                    let mut position = sub.start;
                    while position < sub.end {
                        let key = self.key(&self.index[position]);
                        let syllable = key[base_len..].split(' ').next().unwrap_or_default();
                        prefix.truncate(base_len);
                        prefix.push_str(syllable);
                        if last {
                            if key.len() == prefix.len() {
                                self.push_matches(&self.index[position], true, out);
                            }
                            prefix.push(' ');
                            position = self
                                .prefix_range(prefix, position..sub.end)
                                .end
                                .max(position + 1);
                            continue;
                        }
                        prefix.push(' ');
                        // 这个音节名下的键：先是可能存在的「正好到此为止」的键，然后是 `音节 + 空格` 开头的一段
                        let block = self.prefix_range(prefix, position..sub.end);
                        self.narrow(positions, depth + 1, prefix, block.clone(), exact_only, out);
                        position = block.end.max(position + 1);
                    }
                }
            }
        }
        prefix.truncate(base_len);
    }

    /// 逐键比对：`range` 里每个键从字节偏移 `offset` 起，第一个音节满足 `first`、后面依次满足 `rest` 之一写法的才收。
    pub(super) fn scan_range<'a, 'p>(
        &'a self,
        first: &SyllablePattern<'p>,
        rest: &[impl AsRef<[SyllablePattern<'p>]>],
        offset: usize,
        range: Range<usize>,
        exact_only: bool,
        out: &mut Vec<Match<'a>>,
    ) {
        for entry in &self.index[range] {
            let mut syllables = self.key(entry)[offset..].split(' ');
            let accepted = syllables.next().is_some_and(|s| first.accepts(s))
                && rest.iter().all(|position| {
                    syllables
                        .next()
                        .is_some_and(|s| position.as_ref().iter().any(|p| p.accepts(s)))
                });
            if accepted {
                let exact = syllables.next().is_none();
                if exact || !exact_only {
                    self.push_matches(entry, exact, out);
                }
            }
        }
    }

    /// `range` 里以 `prefix` 开头的键所在的子区间（可能为空）。
    pub(super) fn prefix_range(&self, prefix: &str, range: Range<usize>) -> Range<usize> {
        let entries = &self.index[range.clone()];
        let start = range.start + entries.partition_point(|entry| self.key(entry) < prefix);
        let end = start
            + self.index[start..range.end]
                .partition_point(|entry| self.key(entry).starts_with(prefix));
        start..end
    }
}
