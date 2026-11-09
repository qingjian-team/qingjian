//! 滑行解码的候选词：小写字母串加一个词频先验，按首尾字母分桶，解码时只看起点、终点附近那几个键开头结尾的词。

use std::collections::HashMap;

use qingjian_dictionary::{Dictionary, WordList};

/// 滑行能解出的全部字母串。空的就没有词可比，解码退回按拐点取字母。
#[derive(Debug, Default)]
pub struct SwipeLexicon {
    /// （字母串，先验）：先验是词频取 log10，同一份词表里比大小就行，不同词表之间不用对齐。
    words: Vec<(Box<str>, f32)>,

    /// 首字母 × 26 + 尾字母 → `words` 里的下标。
    buckets: Vec<Vec<u32>>,
}

impl SwipeLexicon {
    /// `(字母串, 先验)`；不全是 a–z 的跳过，同一串取先验大的。
    pub fn new(words: impl IntoIterator<Item = (String, f32)>) -> Self {
        let mut best: HashMap<String, f32> = HashMap::new();
        for (code, prior) in words {
            if code.is_empty() || !code.bytes().all(|b| b.is_ascii_lowercase()) {
                continue;
            }
            let slot = best.entry(code).or_insert(f32::NEG_INFINITY);
            *slot = slot.max(prior);
        }
        let mut words: Vec<(Box<str>, f32)> = best
            .into_iter()
            .map(|(code, prior)| (code.into_boxed_str(), prior))
            .collect();
        words.sort_by(|a, b| a.0.cmp(&b.0));
        let mut buckets = vec![Vec::new(); 26 * 26];
        for (index, (code, _)) in words.iter().enumerate() {
            buckets[bucket(code.as_bytes())].push(index as u32);
        }
        Self { words, buckets }
    }

    /// 英文词表：词频按随包词表的约定是 Zipf 频率 × 1000（已经是 log10），除回去就是先验。
    pub fn from_word_list(list: &WordList) -> Self {
        Self::new(
            list.entries()
                .map(|(code, _, frequency)| (code.to_owned(), frequency as f32 / 1000.0)),
        )
    }

    /// 拼音词库：每个词的音节连起来（`zhong guo` → `zhongguo`），同一串的各个词词频相加再取 log10。
    /// 单音节也在里面，所以只划一个音节也解得出。
    pub fn from_dictionary(dictionary: &Dictionary) -> Self {
        let mut sums: HashMap<String, u64> = HashMap::new();
        for entry in dictionary.entries() {
            let code: String = entry.syllables().collect();
            *sums.entry(code).or_default() += u64::from(entry.frequency);
        }
        Self::new(
            sums.into_iter()
                .map(|(code, sum)| (code, ((sum + 1) as f32).log10())),
        )
    }

    pub fn len(&self) -> usize {
        self.words.len()
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    /// 以 `first` 开头、`last` 结尾的词：（字母串，先验）。
    pub(super) fn ending(&self, first: u8, last: u8) -> impl Iterator<Item = (&str, f32)> {
        let indices = self
            .buckets
            .get(bucket(&[first, last]))
            .into_iter()
            .flatten();
        indices.map(|&index| {
            let (code, prior) = &self.words[index as usize];
            (&**code, *prior)
        })
    }
}

/// 字母串（已知非空、全是 a–z）的桶号。
fn bucket(code: &[u8]) -> usize {
    let first = code[0] - b'a';
    let last = code[code.len() - 1] - b'a';
    first as usize * 26 + last as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_letters_only_and_the_larger_prior() {
        let lexicon = SwipeLexicon::new([
            ("the".to_owned(), 7.0),
            ("the".to_owned(), 5.0),
            ("don't".to_owned(), 6.0),
            ("Hi".to_owned(), 6.0),
            (String::new(), 1.0),
        ]);
        assert_eq!(lexicon.len(), 1);
        assert_eq!(
            lexicon.ending(b't', b'e').collect::<Vec<_>>(),
            vec![("the", 7.0)]
        );
        assert_eq!(lexicon.ending(b't', b'h').count(), 0);
    }

    #[test]
    fn pinyin_joins_syllables_and_sums_frequencies() {
        let dictionary =
            Dictionary::parse("中国\tzhong guo\t900\n种果\tzhong guo\t99\n你\tni\t9\n").unwrap();
        let lexicon = SwipeLexicon::from_dictionary(&dictionary);
        let zhongguo: Vec<_> = lexicon.ending(b'z', b'o').collect();
        assert_eq!(zhongguo[0].0, "zhongguo");
        assert!((zhongguo[0].1 - 3.0).abs() < 1e-4);
        assert_eq!(lexicon.ending(b'n', b'i').count(), 1);
    }
}
