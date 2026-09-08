//! 汉字 → 全拼：按语言模型把句子切成词（[`segment_text`]），每个词查词库里的读音，
//! 词库没有的词退到逐字查；多音字取词库里那个词（或那个字最常用）的读音。任何一个字查不到就放弃这句。

use std::collections::HashMap;

use qingjian_core::sentence::{LanguageModel, segment_text};
use qingjian_dictionary::Dictionary;

/// 汉字到读音的反查表。
pub struct Transcriber {
    /// 词文本 → 无分隔全拼；同一个词多个读音时取词频最高的。
    readings: HashMap<String, (String, u32)>,
}

impl Transcriber {
    /// 从词库（主词库 + 附加词库）建反查表。
    pub fn new<'a>(dictionaries: impl IntoIterator<Item = &'a Dictionary>) -> Self {
        let mut readings: HashMap<String, (String, u32)> = HashMap::new();
        for dictionary in dictionaries {
            for entry in dictionary.entries() {
                let pinyin: String = entry.pinyin.split(' ').collect();
                match readings.get_mut(entry.text) {
                    Some(best) if best.1 >= entry.frequency => {}
                    Some(best) => *best = (pinyin, entry.frequency),
                    None => {
                        readings.insert(entry.text.to_owned(), (pinyin, entry.frequency));
                    }
                }
            }
        }
        Self { readings }
    }

    pub fn len(&self) -> usize {
        self.readings.len()
    }

    /// 一句纯汉字的全拼；有字查不到读音就 `None`。
    pub fn transcribe(&self, text: &str, model: &dyn LanguageModel) -> Option<String> {
        let words: Vec<String> = match segment_text(text, model) {
            Some(clauses) => clauses.into_iter().flatten().collect(),
            None => text.chars().map(String::from).collect(),
        };
        let mut pinyin = String::new();
        for word in &words {
            match self.readings.get(word) {
                Some((reading, _)) => pinyin.push_str(reading),
                None => {
                    for c in word.chars() {
                        let (reading, _) = self.readings.get(c.encode_utf8(&mut [0; 4]))?;
                        pinyin.push_str(reading);
                    }
                }
            }
        }
        Some(pinyin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qingjian_core::sentence::NoLanguageModel;

    const SAMPLE: &str =
        "长\tzhang\t900\n长\tchang\t800\n大\tda\t1000\n长度\tchang du\t500\n度\tdu\t700\n";

    #[test]
    fn prefers_the_word_reading_over_the_most_common_char_reading() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let transcriber = Transcriber::new([&dictionary]);
        // 没有语言模型时逐字：长 取词频高的 zhang
        assert_eq!(
            transcriber.transcribe("长大", &NoLanguageModel).as_deref(),
            Some("zhangda")
        );
        // 词库有「长度」这个词：整词查到 chang
        struct Model;
        impl LanguageModel for Model {
            fn log_prob(&self, _: Option<&str>, word: &str) -> Option<f64> {
                (word == "长度").then_some(-3.0)
            }
        }
        assert_eq!(
            transcriber.transcribe("长度", &Model).as_deref(),
            Some("changdu")
        );
        assert_eq!(transcriber.transcribe("长短", &NoLanguageModel), None);
    }
}
