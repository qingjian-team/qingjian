use std::collections::HashSet;
use std::path::Path;

use qingjian_core::sentence::LanguageModel;
use qingjian_format::{Container, Kind, Metadata, Table, Text, Writer, hash};

use crate::error::LmError;
use crate::successor::Successor;
use crate::word_entry::WordEntry;

/// 句首标记，语料统计时每个句子的第一个词都跟在它后面。
pub const SENTENCE_START: &str = "<s>";

/// 二元概率里 bigram 部分的权重，其余给一元概率。
const LAMBDA: f64 = 0.8;

/// `.qj` 里的分节：词文本 arena、词表、词的哈希索引、CSR 的段偏移与后继。
const WORDS_TAG: [u8; 4] = *b"WORD";
const ENTRIES_TAG: [u8; 4] = *b"ENTR";
const HASH_TAG: [u8; 4] = *b"HASH";
const OFFSETS_TAG: [u8; 4] = *b"OFFS";
const SUCCESSORS_TAG: [u8; 4] = *b"SUCC";

/// 词级 bigram 模型。内存布局就是文件布局：
/// 词表是 arena + 定长条目 + 开放寻址哈希索引（`qingjian_format::hash`），
/// bigram 按前词分组成 CSR（前词 `v` 的后继在 `successors[offsets[v]..offsets[v+1]]`，段内按后词编号升序）。
/// 整句转换每一步都要查它：扁平数组二分一次要跳二十来个缓存行，分组之后一段通常就几条，落在一两个缓存行里。
#[derive(Debug, Default)]
pub struct BigramModel {
    /// 所有词首尾相接。
    words: Text,

    /// 词表，下标即编号。
    entries: Table<WordEntry>,

    /// 词 → 编号的哈希索引。
    index: Table<u32>,

    /// CSR 段偏移，长度 = 词数 + 1。
    offsets: Table<u32>,

    /// CSR 后继。
    successors: Table<Successor>,

    /// 一元计数总和（不含句首标记）。
    total: f64,

    /// 句首标记的编号。
    start: Option<u32>,

    /// `.qj` 里的来历。
    metadata: Option<Metadata>,
}

impl BigramModel {
    /// 从两个 TSV 解析。
    pub fn from_paths(unigram: &Path, bigram: &Path) -> Result<Self, LmError> {
        Self::parse(
            &std::fs::read_to_string(unigram)?,
            &std::fs::read_to_string(bigram)?,
        )
    }

    /// 打开 `.qj` 语言模型。
    pub fn from_path(path: &Path) -> Result<Self, LmError> {
        let container = Container::open(path, Kind::LanguageModel)?;
        let words = container.text(WORDS_TAG)?;
        let entries: Table<WordEntry> = container.table(ENTRIES_TAG)?;
        let index: Table<u32> = container.table(HASH_TAG)?;
        let offsets: Table<u32> = container.table(OFFSETS_TAG)?;
        let successors: Table<Successor> = container.table(SUCCESSORS_TAG)?;
        let count = entries.len();
        for entry in entries.iter() {
            let start = entry.text_start as usize;
            if words
                .get(start..start + usize::from(entry.text_len))
                .is_none()
            {
                return Err(LmError::Corrupt("word entry points outside the arena"));
            }
        }
        if !hash::is_valid(&index, count) {
            return Err(LmError::Corrupt("hash index does not match the word table"));
        }
        if offsets.len() != count + 1
            || offsets.windows(2).any(|w| w[0] > w[1])
            || offsets
                .last()
                .is_some_and(|&end| end as usize != successors.len())
        {
            return Err(LmError::Corrupt("CSR offsets are inconsistent"));
        }
        if successors.iter().any(|s| s.word as usize >= count) {
            return Err(LmError::Corrupt("successor points outside the word table"));
        }
        let mut model = Self {
            words,
            entries,
            index,
            offsets,
            successors,
            total: 0.0,
            start: None,
            metadata: Some(container.metadata().clone()),
        };
        model.finish();
        tracing::debug!(
            words = model.word_count(),
            bigrams = model.bigram_count(),
            name = %container.metadata().name,
            "语言模型已映射"
        );
        Ok(model)
    }

    /// 解析 TSV：一元表按出现顺序编号，二元表排序后分组成 CSR，词表建哈希索引。
    pub fn parse(unigram: &str, bigram: &str) -> Result<Self, LmError> {
        let mut words = String::new();
        let mut entries: Vec<WordEntry> = Vec::new();
        let mut seen: HashSet<&str> = HashSet::new();
        for (index, raw) in unigram.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (word, count) = line.split_once('\t').ok_or(LmError::Format {
                file: "lm-unigram.tsv",
                line: index + 1,
                reason: "expected word<TAB>count",
            })?;
            let count: u32 = count.trim().parse().map_err(|_| LmError::Format {
                file: "lm-unigram.tsv",
                line: index + 1,
                reason: "count is not a number",
            })?;
            let text_len = u16::try_from(word.len()).map_err(|_| LmError::Format {
                file: "lm-unigram.tsv",
                line: index + 1,
                reason: "word too long",
            })?;
            // 重复的词只认第一次
            if !seen.insert(word) {
                continue;
            }
            entries.push(WordEntry {
                text_start: words.len() as u32,
                count,
                text_len,
                reserved: 0,
            });
            words.push_str(word);
        }
        let index = build_index(&words, &entries);
        let lookup =
            |word: &str| hash::find(&index, word, |id| word_text(&words, &entries[id as usize]));
        let mut pairs: Vec<(u32, u32, u32)> = Vec::new();
        for (line_number, raw) in bigram.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            let (Some(first), Some(second), Some(count)) =
                (fields.next(), fields.next(), fields.next())
            else {
                return Err(LmError::Format {
                    file: "lm-bigram.tsv",
                    line: line_number + 1,
                    reason: "expected word<TAB>word<TAB>count",
                });
            };
            let count: u32 = count.trim().parse().map_err(|_| LmError::Format {
                file: "lm-bigram.tsv",
                line: line_number + 1,
                reason: "count is not a number",
            })?;
            // 一元表里没有的词直接跳过：没有 c(v) 也算不出条件概率
            let (Some(first), Some(second)) = (lookup(first), lookup(second)) else {
                continue;
            };
            pairs.push((first, second, count));
        }
        pairs.sort_unstable_by_key(|&(v, w, _)| (v, w));
        pairs.dedup_by_key(|&mut (v, w, _)| (v, w));
        let (offsets, successors) = group(&pairs, entries.len());
        let mut model = Self {
            words: Text::Owned(words),
            entries: Table::Owned(entries),
            index: Table::Owned(index),
            offsets: Table::Owned(offsets),
            successors: Table::Owned(successors),
            total: 0.0,
            start: None,
            metadata: None,
        };
        model.finish();
        tracing::debug!(
            words = model.word_count(),
            bigrams = model.bigram_count(),
            "语言模型加载完成"
        );
        Ok(model)
    }

    /// 写成 `.qj`。`metadata.entries` 会填成 bigram 对数。
    pub fn write_qj(&self, path: &Path, metadata: &Metadata) -> Result<(), LmError> {
        let metadata = Metadata {
            entries: self.successors.len() as u64,
            ..metadata.clone()
        };
        Writer::new(Kind::LanguageModel, &metadata)?
            .section(WORDS_TAG, self.words.as_bytes())
            .section(ENTRIES_TAG, self.entries.as_bytes())
            .section(HASH_TAG, self.index.as_bytes())
            .section(OFFSETS_TAG, self.offsets.as_bytes())
            .section(SUCCESSORS_TAG, self.successors.as_bytes())
            .write_to(path)?;
        Ok(())
    }

    /// 四段数据就位后算派生量：总计数与句首编号。
    fn finish(&mut self) {
        self.start = self.word_id(SENTENCE_START);
        self.total = self
            .entries
            .iter()
            .enumerate()
            .filter(|&(id, _)| Some(id as u32) != self.start)
            .map(|(_, e)| f64::from(e.count))
            .sum::<f64>()
            .max(1.0);
    }

    pub fn word_count(&self) -> usize {
        self.entries.len()
    }

    pub fn bigram_count(&self) -> usize {
        self.successors.len()
    }

    /// `.qj` 里的来历；TSV 解析的返回 `None`。
    pub fn metadata(&self) -> Option<&Metadata> {
        self.metadata.as_ref()
    }

    fn word_id(&self, word: &str) -> Option<u32> {
        hash::find(&self.index, word, |id| {
            word_text(&self.words, &self.entries[id as usize])
        })
    }

    fn bigram(&self, previous: u32, word: u32) -> Option<u32> {
        let start = *self.offsets.get(previous as usize)? as usize;
        let end = *self.offsets.get(previous as usize + 1)? as usize;
        let successors = &self.successors[start..end];
        successors
            .binary_search_by_key(&word, |s| s.word)
            .ok()
            .map(|index| successors[index].count)
    }
}

/// 条目对应的词。
fn word_text<'a>(words: &'a str, entry: &WordEntry) -> &'a str {
    let start = entry.text_start as usize;
    &words[start..start + usize::from(entry.text_len)]
}

fn build_index(words: &str, entries: &[WordEntry]) -> Vec<u32> {
    hash::build(entries.len(), |id| word_text(words, &entries[id as usize]))
}

/// 把按 (前词, 后词) 排好的三元组分成每个前词一段。
fn group(pairs: &[(u32, u32, u32)], word_count: usize) -> (Vec<u32>, Vec<Successor>) {
    let mut offsets = vec![0u32; word_count + 1];
    let mut successors = Vec::with_capacity(pairs.len());
    for &(previous, word, count) in pairs {
        offsets[previous as usize + 1] += 1;
        successors.push(Successor { word, count });
    }
    for index in 1..offsets.len() {
        offsets[index] += offsets[index - 1];
    }
    (offsets, successors)
}

impl LanguageModel for BigramModel {
    fn log_prob(&self, previous: Option<&str>, word: &str) -> Option<f64> {
        let id = self.word_id(word)?;
        let unigram = f64::from(self.entries[id as usize].count) / self.total;
        let previous = match previous {
            None => self.start,
            Some(text) => self.word_id(text),
        };
        let probability = match previous {
            Some(prev_id) if self.entries[prev_id as usize].count > 0 => {
                let pair = self.bigram(prev_id, id).map_or(0.0, f64::from);
                LAMBDA * pair / f64::from(self.entries[prev_id as usize].count)
                    + (1.0 - LAMBDA) * unigram
            }
            // 前词不在模型里：只剩一元概率
            _ => unigram,
        };
        Some(probability.max(f64::MIN_POSITIVE).ln())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNIGRAM: &str = "<s>\t100\n我\t50\n想\t30\n去\t20\n翔\t1\n";
    const BIGRAM: &str = "<s>\t我\t40\n我\t想\t25\n想\t去\t15\n我\t翔\t1\n";

    #[test]
    fn bigram_beats_unigram_and_unknown_words_are_none() {
        let model = BigramModel::parse(UNIGRAM, BIGRAM).unwrap();
        assert_eq!(model.word_count(), 5);
        assert_eq!(model.bigram_count(), 4);
        let xiang = model.log_prob(Some("我"), "想").unwrap();
        let qu = model.log_prob(Some("我"), "去").unwrap();
        assert!(xiang > qu);
        // 句首概率
        assert!(model.log_prob(None, "我").unwrap() > model.log_prob(None, "去").unwrap());
        // 前词未知只用一元
        let alone = model.log_prob(Some("火星"), "想").unwrap();
        assert!((alone - (30.0_f64 / 101.0).ln()).abs() < 1e-9);
        assert_eq!(model.log_prob(Some("我"), "火星"), None);
    }

    #[test]
    fn qj_round_trip_gives_identical_probabilities() {
        let model = BigramModel::parse(UNIGRAM, BIGRAM).unwrap();
        let dir = std::env::temp_dir().join("qingjian-lm-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("lm-{}.qj", std::process::id()));
        let metadata = Metadata {
            name: "测试模型".to_owned(),
            ..Metadata::default()
        };
        model.write_qj(&path, &metadata).unwrap();
        let mapped = BigramModel::from_path(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(mapped.word_count(), model.word_count());
        assert_eq!(mapped.bigram_count(), model.bigram_count());
        assert_eq!(mapped.metadata().unwrap().entries, 4);
        for (previous, word) in [
            (None, "我"),
            (Some("我"), "想"),
            (Some("我"), "去"),
            (Some("想"), "去"),
            (Some("火星"), "想"),
            (Some("我"), "火星"),
        ] {
            assert_eq!(
                mapped.log_prob(previous, word),
                model.log_prob(previous, word)
            );
        }
    }
}
