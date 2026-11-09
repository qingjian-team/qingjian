//! 翻译组的译文缓存：删了又打回去、换个语言再换回来都不重发。只在内存里，换模型时清空；
//! 框里是用户自己打的句子，不落盘。

use std::collections::{HashMap, VecDeque};

use super::TranslateJob;

/// 最多记几条，超了丢最早的。
const CAPACITY: usize = 300;

#[derive(Debug, Default)]
pub(crate) struct TranslationCache {
    entries: HashMap<TranslateJob, String>,

    /// 放进来的先后，满了从头丢。
    order: VecDeque<TranslateJob>,
}

impl TranslationCache {
    pub(crate) fn get(&self, job: &TranslateJob) -> Option<&str> {
        self.entries.get(job).map(String::as_str)
    }

    pub(crate) fn insert(&mut self, job: TranslateJob, text: String) {
        if self.entries.insert(job.clone(), text).is_some() {
            return;
        }
        self.order.push_back(job);
        if self.order.len() > CAPACITY
            && let Some(oldest) = self.order.pop_front()
        {
            self.entries.remove(&oldest);
        }
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Language;

    fn job(source: &str) -> TranslateJob {
        TranslateJob {
            source: source.to_owned(),
            language: Language::English,
        }
    }

    #[test]
    fn drops_the_oldest_when_full() {
        let mut cache = TranslationCache::default();
        for i in 0..=CAPACITY {
            cache.insert(job(&i.to_string()), i.to_string());
        }
        assert_eq!(cache.get(&job("0")), None);
        assert_eq!(cache.get(&job("1")), Some("1"));
        assert_eq!(cache.entries.len(), CAPACITY);
    }
}
