//! 整句头每个音节能出哪些字：模型说明规定只在这个音节读得出的字里比较。

use std::collections::HashMap;

/// 拼音音节 → 读得出它的字在字表里的列号，来自 `unified_vocab.json` 的 `readings`（字 → 读音）。
pub(super) struct Readings {
    columns: HashMap<String, Vec<u32>>,
}

impl Readings {
    pub fn new(readings: &HashMap<String, Vec<String>>, chars: &HashMap<char, u32>) -> Self {
        let mut columns: HashMap<String, Vec<u32>> = HashMap::new();
        for (text, syllables) in readings {
            let Some(&column) = text.chars().next().and_then(|ch| chars.get(&ch)) else {
                continue;
            };
            for syllable in syllables {
                columns.entry(syllable.clone()).or_default().push(column);
            }
        }
        Self { columns }
    }

    /// 敲全的音节取读这个音的字，没敲全的取读音以它开头的字；一个都没有返回 `None`（模型包没有 `readings`）。
    pub fn allowed(&self, text: &str, complete: bool) -> Option<Vec<u32>> {
        let mut out: Vec<u32> = if complete {
            self.columns.get(text)?.clone()
        } else {
            self.columns
                .iter()
                .filter(|(syllable, _)| syllable.starts_with(text))
                .flat_map(|(_, columns)| columns.iter().copied())
                .collect()
        };
        out.sort_unstable();
        out.dedup();
        (!out.is_empty()).then_some(out)
    }
}
