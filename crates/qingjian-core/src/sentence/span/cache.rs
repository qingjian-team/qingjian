use std::collections::HashMap;
use std::sync::Arc;

use qingjian_dictionary::SyllablePattern;

use super::word::SpanWord;

/// 缓存最多存多少个格子，超过就整个清掉（一次整句最多几百个格子，正常输入远到不了）。
pub const MAX_SPAN_CACHE_ENTRIES: usize = 8192;

/// 词图格子候选的缓存：键是格子的模式（各位置的写法与是否完整），值是排好、截好的候选。
///
/// 敲键是增量的：第 n+1 键只新增以它结尾的几个格子，其余格子上一键都算过，而格子查词是整句转换里最贵的一步
/// （每个简拼位置都要在词库里逐音节块收窄）。候选与词库、用户词、用户选择次数、个人出现次数有关，
/// 这些一变（上屏 / 学习）就整个清掉，由 Engine 负责。
#[derive(Debug, Default)]
pub struct SpanCache {
    /// 格子模式 → 候选。
    entries: HashMap<String, Arc<[SpanWord]>>,
}

impl SpanCache {
    /// 格子的缓存键：每个位置的各种写法用 `|` 连、前缀写法带 `…`，位置之间空格。
    pub fn key(span: &[Vec<SyllablePattern<'_>>]) -> String {
        let mut key = String::with_capacity(span.len() * 8);
        for (index, position) in span.iter().enumerate() {
            if index > 0 {
                key.push(' ');
            }
            for (alt, pattern) in position.iter().enumerate() {
                if alt > 0 {
                    key.push('|');
                }
                key.push_str(pattern.text);
                if !pattern.complete {
                    key.push('…');
                }
            }
        }
        key
    }

    /// 有就直接给，没有就算一次存起来。
    pub fn get_or_insert_with(
        &mut self,
        key: String,
        compute: impl FnOnce() -> Vec<SpanWord>,
    ) -> Arc<[SpanWord]> {
        if let Some(found) = self.entries.get(&key) {
            return Arc::clone(found);
        }
        if self.entries.len() >= MAX_SPAN_CACHE_ENTRIES {
            self.entries.clear();
        }
        let words: Arc<[SpanWord]> = compute().into();
        self.entries.insert(key, Arc::clone(&words));
        words
    }

    /// 词库、用户词或学习数据变了：全部作废。
    pub fn clear(&mut self) {
        self.entries.clear();
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
    fn key_distinguishes_completeness_and_alternatives() {
        let full = vec![vec![SyllablePattern::complete("kai")]];
        let prefix = vec![vec![SyllablePattern::prefix("kai")]];
        let fuzzy = vec![vec![
            SyllablePattern::prefix("z"),
            SyllablePattern::prefix("zh"),
        ]];
        assert_eq!(SpanCache::key(&full), "kai");
        assert_eq!(SpanCache::key(&prefix), "kai…");
        assert_eq!(SpanCache::key(&fuzzy), "z…|zh…");
        assert_ne!(SpanCache::key(&full), SpanCache::key(&prefix));
    }

    #[test]
    fn second_lookup_is_served_from_the_cache() {
        let mut cache = SpanCache::default();
        let mut computed = 0;
        let mut fetch = |cache: &mut SpanCache| {
            cache.get_or_insert_with("kai fa".to_owned(), || {
                computed += 1;
                vec![SpanWord {
                    text: "开发".to_owned(),
                    syllables: vec!["kai".to_owned(), "fa".to_owned()],
                    frequency: 9,
                    penalty: 0.0,
                }]
            })
        };
        let first = fetch(&mut cache);
        let second = fetch(&mut cache);
        assert_eq!(first, second);
        assert_eq!(computed, 1);
        assert_eq!(cache.len(), 1);
        cache.clear();
        assert!(cache.is_empty());
    }
}
