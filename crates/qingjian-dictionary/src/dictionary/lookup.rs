//! 词库的音节模式查询：逐音节二分收窄区间，区间足够小时退回线性扫描。

use std::ops::Range;

use crate::matching::Match;
use crate::pattern::{SyllablePattern, canonical_syllable};

use super::{Dictionary, KeyIndex, LINEAR_SCAN_LIMIT};

impl Dictionary {
    /// 只查音节数与模式长度**恰好相等**的词：整句转换的词图每个格子只要正好覆盖这几个音节的词，
    /// 输入前缀出候选也只要这种。与 [`Self::lookup_pattern`] 同一套收窄，只是最后一级不收更长的词、
    /// 简拼位置只看每个音节块的第一条键，代价与匹配到的音节组合数成正比。
    pub fn lookup_exact(&self, pattern: &[SyllablePattern<'_>]) -> Vec<Match<'_>> {
        let positions: Vec<&[SyllablePattern<'_>]> =
            pattern.iter().map(std::slice::from_ref).collect();
        self.lookup_positions(&positions, true, None)
    }

    /// 按音节序列查词。
    ///
    /// - 完整音节：命中音节序列相同的词，以及以该序列为前缀的更长词。
    /// - `last_is_partial` 为真时，最后一个音节按前缀匹配（如 `kai f` 命中 `kai fa`、`kai fang`）。
    pub fn lookup(&self, syllables: &[&str], last_is_partial: bool) -> Vec<Match<'_>> {
        let count = syllables.len();
        let pattern: Vec<SyllablePattern<'_>> = syllables
            .iter()
            .enumerate()
            .map(|(i, s)| SyllablePattern {
                text: s,
                complete: !(last_is_partial && i + 1 == count),
            })
            .collect();
        self.lookup_pattern(&pattern)
    }

    /// 按模式查词：每个位置可以是完整音节，也可以是前缀 / 声母（简拼）。
    /// 命中音节数 ≥ 模式长度、逐位满足模式的词。
    pub fn lookup_pattern(&self, pattern: &[SyllablePattern<'_>]) -> Vec<Match<'_>> {
        let positions: Vec<&[SyllablePattern<'_>]> =
            pattern.iter().map(std::slice::from_ref).collect();
        self.lookup_positions(&positions, false, None)
    }

    /// 每个位置给多种写法（模糊音：`zi` 也接受 `zhi`），命中音节数 ≥ 位置数、逐位满足其中一种写法的词。
    ///
    /// 同一位置的写法之间不能互相覆盖（前缀 `z` 已包含前缀 `zh`，两者不要同时给；同一写法不要重复），
    /// 否则命中会重复出现。调用方按这个契约生成写法，这里不去重。
    pub fn lookup_pattern_alt<'p>(
        &self,
        positions: &[impl AsRef<[SyllablePattern<'p>]>],
    ) -> Vec<Match<'_>> {
        self.lookup_positions(positions, false, None)
    }

    /// [`Self::lookup_exact`] 的多写法版。
    pub fn lookup_exact_alt<'p>(
        &self,
        positions: &[impl AsRef<[SyllablePattern<'p>]>],
    ) -> Vec<Match<'_>> {
        self.lookup_positions(positions, true, None)
    }

    /// 同 [`Self::lookup_exact_alt`]，同时返回词库里是否还有以这串位置为前缀的更长词（`has_longer`）。
    /// 词图在字节偏移 DAG 上按音节数向外扩展时，用它决定要不要继续往下走：没有更长词就原地剪枝。
    pub fn lookup_exact_alt_step<'p>(
        &self,
        positions: &[impl AsRef<[SyllablePattern<'p>]>],
    ) -> (Vec<Match<'_>>, bool) {
        let mut has_longer = false;
        let matches = self.lookup_positions(positions, true, Some(&mut has_longer));
        (matches, has_longer)
    }

    /// 键按字节序排好，所以「以某段前缀开头的键」总是一段连续区间：逐个音节位置用二分收窄区间，
    /// 完整音节直接定位到 `前缀 + 音节 + 空格`，简拼位置按区间里出现的不同音节跳着扫（每个音节只看一条键就能算出它的子区间）。
    /// 代价只和匹配到的音节组合数成正比，与首音节下有多少键无关；一个位置的多种写法逐个走，代价相加而不相乘。
    fn lookup_positions<'p>(
        &self,
        positions: &[impl AsRef<[SyllablePattern<'p>]>],
        exact_only: bool,
        has_longer: Option<&mut bool>,
    ) -> Vec<Match<'_>> {
        let mut matches = Vec::new();
        if positions.is_empty() || positions.iter().any(|p| p.as_ref().is_empty()) {
            return matches;
        }
        let mut prefix = String::with_capacity(positions.len() * 6);
        self.narrow(
            positions,
            0,
            &mut prefix,
            0..self.index.len(),
            exact_only,
            has_longer,
            &mut matches,
        );
        matches
    }

    /// 在 `range`（其中的键都以 `prefix` 开头，`prefix` 为空或以空格结尾）里按 `positions[depth..]` 继续收窄。
    /// `exact_only` 为真时只收音节数正好等于位置数的词；`has_longer` 非空时记录是否还存在更长的词。
    #[allow(clippy::too_many_arguments)]
    fn narrow<'a, 'p>(
        &'a self,
        positions: &[impl AsRef<[SyllablePattern<'p>]>],
        depth: usize,
        prefix: &mut String,
        range: Range<usize>,
        exact_only: bool,
        mut has_longer: Option<&mut bool>,
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
            if sub.len() <= LINEAR_SCAN_LIMIT {
                self.scan_range(
                    current,
                    &positions[depth + 1..],
                    base_len,
                    sub,
                    exact_only,
                    has_longer.as_deref_mut(),
                    out,
                );
                continue;
            }
            match (current.complete, last) {
                // 完整且是最后一个：正好这个键的词精确命中，`键 + 空格` 开头的更长词也收
                (true, true) => {
                    let exact_entry = self.index[sub.clone()]
                        .first()
                        .filter(|entry| self.key(entry).len() == prefix.len());
                    if let Some(entry) = exact_entry {
                        self.push_matches(entry, true, out);
                    }
                    if !exact_only {
                        prefix.push(' ');
                        for entry in &self.index[self.prefix_range(prefix, sub)] {
                            self.push_matches(entry, false, out);
                        }
                    } else if !self.chars_only
                        && let Some(longer) = has_longer.as_deref_mut()
                        && !*longer
                    {
                        // 键按字节序升序且 `' '` < `'a'`：紧跟精确键（或区间首位）的第一条更长键若不是以空格接续，后面也不可能有
                        let next = sub.start + usize::from(exact_entry.is_some());
                        if next < sub.end
                            && self.key(&self.index[next]).as_bytes().get(prefix.len())
                                == Some(&b' ')
                        {
                            *longer = true;
                        }
                    }
                }
                (true, false) => {
                    prefix.push(' ');
                    let deeper = self.prefix_range(prefix, sub);
                    self.narrow(
                        positions,
                        depth + 1,
                        prefix,
                        deeper,
                        exact_only,
                        has_longer.as_deref_mut(),
                        out,
                    );
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
                            let block = self.prefix_range(prefix, position..sub.end);
                            if !block.is_empty()
                                && !self.chars_only
                                && let Some(longer) = has_longer.as_deref_mut()
                            {
                                *longer = true;
                            }
                            position = block.end.max(position + 1);
                            continue;
                        }
                        prefix.push(' ');
                        // 这个音节名下的键：先是可能存在的「正好到此为止」的键，然后是 `音节 + 空格` 开头的一段
                        let block = self.prefix_range(prefix, position..sub.end);
                        self.narrow(
                            positions,
                            depth + 1,
                            prefix,
                            block.clone(),
                            exact_only,
                            has_longer.as_deref_mut(),
                            out,
                        );
                        position = block.end.max(position + 1);
                    }
                }
            }
        }
        prefix.truncate(base_len);
    }

    /// 逐键比对：`range` 里每个键从字节偏移 `offset` 起，第一个音节满足 `first`、后面依次满足 `rest` 之一写法的才收。
    #[allow(clippy::too_many_arguments)]
    fn scan_range<'a, 'p>(
        &'a self,
        first: &SyllablePattern<'p>,
        rest: &[impl AsRef<[SyllablePattern<'p>]>],
        offset: usize,
        range: Range<usize>,
        exact_only: bool,
        mut has_longer: Option<&mut bool>,
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
                if !exact
                    && !self.chars_only
                    && let Some(longer) = has_longer.as_deref_mut()
                {
                    *longer = true;
                }
            }
        }
    }

    /// `range` 里以 `prefix` 开头的键所在的子区间（可能为空）。
    fn prefix_range(&self, prefix: &str, range: Range<usize>) -> Range<usize> {
        let entries = &self.index[range.clone()];
        let start = range.start + entries.partition_point(|entry| self.key(entry) < prefix);
        let end = start
            + self.index[start..range.end]
                .partition_point(|entry| self.key(entry).starts_with(prefix));
        start..end
    }

    /// 把一个键下的全部词目追加为命中。
    fn push_matches<'a>(&'a self, entry: &KeyIndex, exact: bool, out: &mut Vec<Match<'a>>) {
        let key = self.key(entry);
        if self.chars_only && key.contains(' ') {
            return;
        }
        let range =
            entry.first_slot as usize..entry.first_slot as usize + entry.slot_count as usize;
        for position in range {
            let slot = &self.slots[position];
            out.push(Match {
                text: self.text(slot),
                pinyin: key,
                frequency: slot.frequency,
                reading_total: self.reading_total(key, self.text(slot)),
                tones: self.tones(position, slot),
                secondary: self.secondary,
                exact,
            });
        }
    }

    /// 线性扫描版，只做测试基准：与 [`Self::lookup_pattern_alt`] 结果必须一致。
    #[cfg(test)]
    pub(super) fn lookup_scan<'p>(&self, positions: &[Vec<SyllablePattern<'p>>]) -> Vec<Match<'_>> {
        let mut matches = Vec::new();
        for entry in self.index.iter() {
            let mut syllables = self.key(entry).split(' ');
            let accepted = positions.iter().all(|position| {
                syllables
                    .next()
                    .is_some_and(|s| position.iter().any(|p| p.accepts(s)))
            });
            if accepted {
                let exact = syllables.next().is_none();
                self.push_matches(entry, exact, &mut matches);
            }
        }
        matches
    }
}
