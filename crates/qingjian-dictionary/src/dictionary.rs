use std::ops::Range;
use std::path::Path;

use qingjian_format::{Container, Kind, Metadata, Table, Text, Writer};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::error::DictionaryError;
use crate::matching::Match;
use crate::pattern::SyllablePattern;

/// 收窄到多小的区间就改成逐键比对。
const LINEAR_SCAN_LIMIT: usize = 48;

/// `.qj` 里的分节：词文本 arena、拼音键 arena、键索引、词目。
const TEXT_TAG: [u8; 4] = *b"TEXT";
const KEYS_TAG: [u8; 4] = *b"KEYS";
const INDEX_TAG: [u8; 4] = *b"INDX";
const SLOTS_TAG: [u8; 4] = *b"SLOT";

/// 一条词目：词文本在 arena 里的位置与词频。12 字节、无填充，原样落盘。
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
struct Slot {
    /// 在 `Dictionary::texts` 里的字节偏移。
    text_start: u32,

    /// 静态词频。
    frequency: u32,

    /// 字节长度。
    text_len: u16,

    /// 对齐用，全零。
    reserved: u16,
}

/// 一个拼音键（如 `kai fa`）及其下的词目范围。16 字节、无填充，原样落盘。
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
struct KeyIndex {
    /// 在 `Dictionary::keys` 里的字节偏移。
    key_start: u32,

    /// 在 `Dictionary::slots` 里的起始下标。
    first_slot: u32,

    /// 词目个数（同音词）。
    slot_count: u32,

    /// 字节长度。
    key_len: u16,

    /// 对齐用，全零。
    reserved: u16,
}

/// 解析阶段的临时行，排序后才归并成 `KeyIndex` + `Slot`。
#[derive(Debug, Clone, Copy)]
struct Row {
    key_start: u32,
    key_len: u16,
    slot: Slot,
}

/// 内存词库。从 TSV 解析时四段数据是自己的，从 `.qj` 打开时是映射文件里的，查询代码不区分。
#[derive(Debug, Default)]
pub struct Dictionary {
    /// 所有词文本首尾相接。
    texts: Text,

    /// 所有拼音键首尾相接（同音词共用一个键）。
    keys: Text,

    /// 按键的字节序升序排列。
    index: Table<KeyIndex>,

    /// 与 `index` 同序；同一个键下按词频降序。
    slots: Table<Slot>,

    /// 全部词频之和，整句转换把词频归一化成概率时用。
    total_frequency: u64,

    /// `.qj` 里的来历（名称、许可证、署名）；TSV 解析的没有。
    metadata: Option<Metadata>,
}

impl Dictionary {
    pub fn parse(source: &str) -> Result<Self, DictionaryError> {
        let mut texts = String::new();
        let mut keys = String::new();
        let mut rows: Vec<Row> = Vec::new();
        for (index, raw) in source.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let number = index + 1;
            let mut fields = line.split('\t');
            let text = fields
                .next()
                .filter(|s| !s.is_empty())
                .ok_or(DictionaryError::Line {
                    line: number,
                    reason: "missing word",
                })?;
            let pinyin = fields
                .next()
                .filter(|s| !s.is_empty())
                .ok_or(DictionaryError::Line {
                    line: number,
                    reason: "missing pinyin",
                })?;
            let frequency = fields
                .next()
                .map(|s| s.trim().parse::<u32>())
                .transpose()
                .map_err(|_| DictionaryError::Line {
                    line: number,
                    reason: "frequency is not a non-negative integer",
                })?
                .unwrap_or(1);
            let text_len = u16::try_from(text.len()).map_err(|_| DictionaryError::Line {
                line: number,
                reason: "word too long",
            })?;
            let key_len = u16::try_from(pinyin.len()).map_err(|_| DictionaryError::Line {
                line: number,
                reason: "pinyin too long",
            })?;
            rows.push(Row {
                key_start: keys.len() as u32,
                key_len,
                slot: Slot {
                    text_start: texts.len() as u32,
                    frequency,
                    text_len,
                    reserved: 0,
                },
            });
            texts.push_str(text);
            // 规范化：音节之间恰好一个空格
            let mut first = true;
            for syllable in pinyin.split_whitespace() {
                if !first {
                    keys.push(' ');
                }
                keys.push_str(syllable);
                first = false;
            }
            let actual = keys.len() as u32 - rows.last().unwrap().key_start;
            rows.last_mut().unwrap().key_len = actual as u16;
        }
        Ok(Self::assemble(texts, keys, rows))
    }

    /// 按键排序、同键归并。
    fn assemble(texts: String, keys: String, mut rows: Vec<Row>) -> Self {
        let key_of = |row: &Row| {
            &keys[row.key_start as usize..row.key_start as usize + row.key_len as usize]
        };
        rows.sort_by(|a, b| {
            key_of(a)
                .cmp(key_of(b))
                .then_with(|| b.slot.frequency.cmp(&a.slot.frequency))
        });
        let mut index: Vec<KeyIndex> = Vec::new();
        let mut slots: Vec<Slot> = Vec::with_capacity(rows.len());
        for row in &rows {
            match index.last_mut() {
                Some(last)
                    if key_of(row)
                        == &keys[last.key_start as usize
                            ..last.key_start as usize + last.key_len as usize] =>
                {
                    last.slot_count += 1;
                }
                _ => index.push(KeyIndex {
                    key_start: row.key_start,
                    first_slot: slots.len() as u32,
                    slot_count: 1,
                    key_len: row.key_len,
                    reserved: 0,
                }),
            }
            slots.push(row.slot);
        }
        let total_frequency = slots.iter().map(|s| u64::from(s.frequency)).sum();
        tracing::debug!(entries = slots.len(), keys = index.len(), "词库加载完成");
        Self {
            texts: Text::Owned(texts),
            keys: Text::Owned(keys),
            index: Table::Owned(index),
            slots: Table::Owned(slots),
            total_frequency,
            metadata: None,
        }
    }

    /// 按文件内容选加载方式：`.qj` 容器直接映射，否则当 TSV 解析。
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, DictionaryError> {
        let path = path.as_ref();
        if Container::is_qj(path) {
            Self::open_qj(path)
        } else {
            Self::parse(&std::fs::read_to_string(path)?)
        }
    }

    /// 打开 `.qj` 词库：映射四个分节，逐条校验偏移落在 arena 内且在字符边界上（坏文件报错而不是查询时 panic）。
    pub fn open_qj(path: &Path) -> Result<Self, DictionaryError> {
        let container = Container::open(path, Kind::Dictionary)?;
        let texts = container.text(TEXT_TAG)?;
        let keys = container.text(KEYS_TAG)?;
        let index: Table<KeyIndex> = container.table(INDEX_TAG)?;
        let slots: Table<Slot> = container.table(SLOTS_TAG)?;
        let mut total_frequency = 0u64;
        for slot in slots.iter() {
            let start = slot.text_start as usize;
            if texts
                .get(start..start + usize::from(slot.text_len))
                .is_none()
            {
                return Err(DictionaryError::Corrupt(
                    "slot points outside the text arena",
                ));
            }
            total_frequency += u64::from(slot.frequency);
        }
        for entry in index.iter() {
            let start = entry.key_start as usize;
            if keys
                .get(start..start + usize::from(entry.key_len))
                .is_none()
            {
                return Err(DictionaryError::Corrupt("key points outside the key arena"));
            }
            let first = entry.first_slot as usize;
            if first + entry.slot_count as usize > slots.len() {
                return Err(DictionaryError::Corrupt(
                    "key points outside the slot table",
                ));
            }
        }
        tracing::debug!(
            entries = slots.len(),
            keys = index.len(),
            name = %container.metadata().name,
            "词库已映射"
        );
        Ok(Self {
            texts,
            keys,
            index,
            slots,
            total_frequency,
            metadata: Some(container.metadata().clone()),
        })
    }

    /// 写成 `.qj`：内存里的四段原样落盘。`metadata.entries` 会填成词条数。
    pub fn write_qj(&self, path: &Path, metadata: &Metadata) -> Result<(), DictionaryError> {
        let metadata = Metadata {
            entries: self.slots.len() as u64,
            ..metadata.clone()
        };
        Writer::new(Kind::Dictionary, &metadata)?
            .section(TEXT_TAG, self.texts.as_bytes())
            .section(KEYS_TAG, self.keys.as_bytes())
            .section(INDEX_TAG, self.index.as_bytes())
            .section(SLOTS_TAG, self.slots.as_bytes())
            .write_to(path)?;
        Ok(())
    }

    /// `.qj` 里的来历；TSV 解析的返回 `None`。
    pub fn metadata(&self) -> Option<&Metadata> {
        self.metadata.as_ref()
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// 全部词频之和。
    pub fn total_frequency(&self) -> u64 {
        self.total_frequency
    }

    /// 全部词目，按拼音键的字节序、同一个键下按词频降序。给反查（汉字 → 读音）建索引用。
    pub fn entries(&self) -> impl Iterator<Item = Match<'_>> + '_ {
        self.index.iter().flat_map(move |entry| {
            let key = self.key(entry);
            let range =
                entry.first_slot as usize..entry.first_slot as usize + entry.slot_count as usize;
            self.slots[range].iter().map(move |slot| Match {
                text: self.text(slot),
                pinyin: key,
                frequency: slot.frequency,
                exact: true,
            })
        })
    }

    /// 只查音节数与模式长度**恰好相等**的词：整句转换的词图每个格子只要正好覆盖这几个音节的词，
    /// 输入前缀出候选也只要这种。与 [`Self::lookup_pattern`] 同一套收窄，只是最后一级不收更长的词、
    /// 简拼位置只看每个音节块的第一条键，代价与匹配到的音节组合数成正比。
    pub fn lookup_exact(&self, pattern: &[SyllablePattern<'_>]) -> Vec<Match<'_>> {
        let positions: Vec<&[SyllablePattern<'_>]> =
            pattern.iter().map(std::slice::from_ref).collect();
        self.lookup_positions(&positions, true)
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
        self.lookup_positions(&positions, false)
    }

    /// 每个位置给多种写法（模糊音：`zi` 也接受 `zhi`），命中音节数 ≥ 位置数、逐位满足其中一种写法的词。
    ///
    /// 同一位置的写法之间不能互相覆盖（前缀 `z` 已包含前缀 `zh`，两者不要同时给；同一写法不要重复），
    /// 否则命中会重复出现。调用方按这个契约生成写法，这里不去重。
    pub fn lookup_pattern_alt<'p>(
        &self,
        positions: &[impl AsRef<[SyllablePattern<'p>]>],
    ) -> Vec<Match<'_>> {
        self.lookup_positions(positions, false)
    }

    /// [`Self::lookup_exact`] 的多写法版。
    pub fn lookup_exact_alt<'p>(
        &self,
        positions: &[impl AsRef<[SyllablePattern<'p>]>],
    ) -> Vec<Match<'_>> {
        self.lookup_positions(positions, true)
    }

    /// 键按字节序排好，所以「以某段前缀开头的键」总是一段连续区间：逐个音节位置用二分收窄区间，
    /// 完整音节直接定位到 `前缀 + 音节 + 空格`，简拼位置按区间里出现的不同音节跳着扫（每个音节只看一条键就能算出它的子区间）。
    /// 代价只和匹配到的音节组合数成正比，与首音节下有多少键无关；一个位置的多种写法逐个走，代价相加而不相乘。
    fn lookup_positions<'p>(
        &self,
        positions: &[impl AsRef<[SyllablePattern<'p>]>],
        exact_only: bool,
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
            &mut matches,
        );
        matches
    }

    /// 在 `range`（其中的键都以 `prefix` 开头，`prefix` 为空或以空格结尾）里按 `positions[depth..]` 继续收窄。
    /// `exact_only` 为真时只收音节数正好等于位置数的词。
    fn narrow<'a, 'p>(
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
            prefix.push_str(current.text);
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
    fn scan_range<'a, 'p>(
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
    fn prefix_range(&self, prefix: &str, range: Range<usize>) -> Range<usize> {
        let entries = &self.index[range.clone()];
        let start = range.start + entries.partition_point(|entry| self.key(entry) < prefix);
        let end = start
            + self.index[start..range.end]
                .partition_point(|entry| self.key(entry).starts_with(prefix));
        start..end
    }

    fn key(&self, entry: &KeyIndex) -> &str {
        &self.keys[entry.key_start as usize..entry.key_start as usize + entry.key_len as usize]
    }

    fn text(&self, slot: &Slot) -> &str {
        &self.texts[slot.text_start as usize..slot.text_start as usize + slot.text_len as usize]
    }

    /// 把一个键下的全部词目追加为命中。
    fn push_matches<'a>(&'a self, entry: &KeyIndex, exact: bool, out: &mut Vec<Match<'a>>) {
        let key = self.key(entry);
        let range =
            entry.first_slot as usize..entry.first_slot as usize + entry.slot_count as usize;
        for slot in &self.slots[range] {
            out.push(Match {
                text: self.text(slot),
                pinyin: key,
                frequency: slot.frequency,
                exact,
            });
        }
    }

    /// 线性扫描版，只做测试基准：与 [`Self::lookup_pattern_alt`] 结果必须一致。
    #[cfg(test)]
    fn lookup_scan<'p>(&self, positions: &[Vec<SyllablePattern<'p>>]) -> Vec<Match<'_>> {
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

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# 测试词库\n开发\tkai fa\t9000\n开发者\tkai fa zhe\t3000\n开饭\tkai fan\t800\n开放\tkai fang\t2000\n咖啡\tka fei\t5000\n开\tkai\t20000\n";

    fn texts<'a>(matches: &'a [Match<'a>]) -> Vec<&'a str> {
        matches.iter().map(|m| m.text).collect()
    }

    #[test]
    fn full_syllables_match_exact_and_longer_words() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let matches = dictionary.lookup(&["kai", "fa"], false);
        assert_eq!(texts(&matches), ["开发", "开发者"]);
        assert!(matches[0].exact);
        assert!(!matches[1].exact);
        assert_eq!(matches[0].pinyin, "kai fa");
    }

    #[test]
    fn exact_lookup_only_returns_words_of_the_same_length() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let exact = dictionary.lookup_exact(&[
            SyllablePattern::complete("kai"),
            SyllablePattern::complete("fa"),
        ]);
        assert_eq!(texts(&exact), ["开发"]);
        let partial = dictionary.lookup_exact(&[
            SyllablePattern::complete("kai"),
            SyllablePattern::prefix("f"),
        ]);
        assert_eq!(texts(&partial), ["开发", "开饭", "开放"]);
        let single = dictionary.lookup_exact(&[SyllablePattern::complete("kai")]);
        assert_eq!(texts(&single), ["开"]);
        assert!(
            dictionary
                .lookup_exact(&[SyllablePattern::complete("ka")])
                .is_empty()
        );
        assert_eq!(
            dictionary.total_frequency(),
            9000 + 3000 + 800 + 2000 + 5000 + 20000
        );
    }

    #[test]
    fn partial_last_syllable_matches_by_prefix() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let matches = dictionary.lookup(&["kai", "fa"], true);
        assert_eq!(texts(&matches), ["开发", "开发者", "开饭", "开放"]);
    }

    #[test]
    fn single_complete_syllable_does_not_match_longer_syllables() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let matches = dictionary.lookup(&["ka"], false);
        assert_eq!(texts(&matches), ["咖啡"]);
        assert!(!matches[0].exact);
    }

    #[test]
    fn initials_pattern_matches_abbreviations() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let matches = dictionary
            .lookup_pattern(&[SyllablePattern::prefix("k"), SyllablePattern::prefix("f")]);
        let mut found = texts(&matches);
        found.sort_unstable();
        assert_eq!(found, ["咖啡", "开发", "开发者", "开放", "开饭"]);
        let mixed = dictionary.lookup_pattern(&[
            SyllablePattern::complete("kai"),
            SyllablePattern::prefix("f"),
        ]);
        assert!(!texts(&mixed).contains(&"咖啡"));
    }

    #[test]
    fn homophones_are_sorted_by_frequency() {
        let dictionary = Dictionary::parse("发\tfa\t100\n法\tfa\t300\n乏\tfa\t50\n").unwrap();
        assert_eq!(
            texts(&dictionary.lookup(&["fa"], false)),
            ["法", "发", "乏"]
        );
        assert_eq!(dictionary.len(), 3);
    }

    #[test]
    fn rejects_malformed_line() {
        let error = Dictionary::parse("开发\tkai fa\tabc\n").unwrap_err();
        assert!(matches!(error, DictionaryError::Line { line: 1, .. }));
    }

    /// 与线性扫描版对拍：随机词库、随机模式（含每个位置多种写法），结果集必须完全一致（顺序按键排序后比较）。
    #[test]
    fn narrowing_agrees_with_linear_scan() {
        const SYLLABLES: [&str; 14] = [
            "a", "an", "ang", "ka", "kai", "kan", "kang", "fa", "fan", "fang", "fei", "shi", "sha",
            "shang",
        ];
        // 简单的线性同余随机数，测试可复现
        let mut state: u64 = 0x2545_F491_4F6C_DD1D;
        let mut next = move |bound: usize| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((state >> 33) as usize) % bound
        };
        let mut tsv = String::new();
        for i in 0..3000 {
            let count = 1 + next(4);
            let key: Vec<&str> = (0..count)
                .map(|_| SYLLABLES[next(SYLLABLES.len())])
                .collect();
            tsv.push_str(&format!("词{i}\t{}\t{}\n", key.join(" "), 1 + next(1000)));
        }
        let dictionary = Dictionary::parse(&tsv).unwrap();
        let random_pattern = |next: &mut dyn FnMut(usize) -> usize| {
            let syllable = SYLLABLES[next(SYLLABLES.len())];
            if next(2) == 0 {
                SyllablePattern::complete(syllable)
            } else {
                SyllablePattern::prefix(&syllable[..1 + next(syllable.len())])
            }
        };
        let by_key = |a: &Match<'_>, b: &Match<'_>| (a.pinyin, a.text).cmp(&(b.pinyin, b.text));
        let mut cases = 0;
        for round in 0..600 {
            let count = 1 + next(4);
            let positions: Vec<Vec<SyllablePattern<'_>>> = (0..count)
                .map(|_| {
                    let alternatives = if round < 300 { 1 } else { 1 + next(3) };
                    let mut position: Vec<SyllablePattern<'_>> = Vec::new();
                    for _ in 0..alternatives {
                        let candidate = random_pattern(&mut next);
                        // 契约：同一位置的写法不能互相覆盖
                        let overlaps = position.iter().any(|p| {
                            (!p.complete && candidate.text.starts_with(p.text))
                                || (!candidate.complete && p.text.starts_with(candidate.text))
                                || p.text == candidate.text
                        });
                        if !overlaps {
                            position.push(candidate);
                        }
                    }
                    position
                })
                .collect();
            let mut slow = dictionary.lookup_scan(&positions);
            slow.sort_by(by_key);
            let mut fast = if round < 300 {
                let single: Vec<SyllablePattern<'_>> = positions.iter().map(|p| p[0]).collect();
                dictionary.lookup_pattern(&single)
            } else {
                dictionary.lookup_pattern_alt(&positions)
            };
            fast.sort_by(by_key);
            assert_eq!(fast, slow, "positions {positions:?}");
            let mut exact = if round < 300 {
                let single: Vec<SyllablePattern<'_>> = positions.iter().map(|p| p[0]).collect();
                dictionary.lookup_exact(&single)
            } else {
                dictionary.lookup_exact_alt(&positions)
            };
            exact.sort_by(by_key);
            slow.retain(|m| m.exact);
            assert_eq!(exact, slow, "exact positions {positions:?}");
            cases += usize::from(!fast.is_empty());
        }
        assert!(cases > 200, "too few non-empty cases: {cases}");
    }

    #[test]
    fn qj_round_trip_preserves_every_lookup() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let dir = std::env::temp_dir().join("qingjian-dictionary-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("dict-{}.qj", std::process::id()));
        let metadata = Metadata {
            name: "测试词库".to_owned(),
            license: "MIT".to_owned(),
            ..Metadata::default()
        };
        dictionary.write_qj(&path, &metadata).unwrap();
        let mapped = Dictionary::from_path(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(mapped.len(), dictionary.len());
        assert_eq!(mapped.total_frequency(), dictionary.total_frequency());
        assert_eq!(mapped.metadata().unwrap().name, "测试词库");
        assert_eq!(mapped.metadata().unwrap().entries, dictionary.len() as u64);
        for (syllables, partial) in [
            (vec!["kai", "fa"], false),
            (vec!["kai"], true),
            (vec!["k", "f"], true),
            (vec!["ka"], true),
        ] {
            let original = dictionary.lookup(&syllables, partial);
            let reopened = mapped.lookup(&syllables, partial);
            assert_eq!(texts(&reopened), texts(&original), "{syllables:?}");
        }
    }
}
