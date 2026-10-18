//! 例句：上屏带译词的词时先记下这个词，这句话写完（句末标点、换行、上文断开、写得太长）再把整句交出去。
//! 句子从上一处句末接着攒，所以词前面的半句也在。只在内存里，私密输入与丢弃输入时清空。
//! 分几次上屏拼成的整句也交回去（生词本时间线要整句翻译）；一次就上屏的整句由上屏那边自己翻。

use crate::candidate::Language;
use crate::sentence::is_han;

/// 句子攒到这么多字还没写完就按这里收：聊天里常常不打句号。
const MAX_SENTENCE: usize = 80;

/// 拼成的整句至少这么多汉字才交回去翻译。
const MIN_SENTENCE_HANZI: usize = 4;

#[derive(Debug, Default)]
pub struct ExampleCollector {
    /// 上一处句末之后上屏的文字。
    sentence: String,

    /// 等这句写完的词：(学习语言, 译词, 带它上屏的中文)。
    waiting: Vec<(Language, String, String)>,

    /// 这句里有几次上屏带汉字。
    pieces: usize,
}

impl ExampleCollector {
    pub fn note_word(&mut self, language: Language, word: &str, source: &str) {
        self.waiting
            .push((language, word.to_owned(), source.to_owned()));
    }

    /// 接上一段上屏文字；每写完一句，对等着的每个词调一次 `emit(语言, 译词, 中文, 例句)`。
    /// 返回写完的、分几次上屏拼成的整句。
    pub fn push_text(
        &mut self,
        text: &str,
        mut emit: impl FnMut(Language, &str, &str, &str),
    ) -> Vec<String> {
        let mut finished = Vec::new();
        if text.chars().any(is_han) {
            self.pieces += 1;
        }
        for c in text.chars() {
            self.sentence.push(c);
            if is_sentence_end(c) {
                finished.extend(self.finish(&mut emit));
            }
        }
        let length = self.sentence.chars().count();
        if length > MAX_SENTENCE {
            if self.waiting.is_empty() {
                // 没有等着的词：只留最近一截当下一句的开头
                let start = self
                    .sentence
                    .char_indices()
                    .nth(length - MAX_SENTENCE)
                    .map_or(0, |(index, _)| index);
                self.sentence.drain(..start);
            } else {
                finished.extend(self.finish(&mut emit));
            }
        }
        finished
    }

    /// 上文断开（换应用、点了别处、停用输入法）：手上这句就算写完了。
    pub fn break_off(
        &mut self,
        mut emit: impl FnMut(Language, &str, &str, &str),
    ) -> Option<String> {
        self.finish(&mut emit)
    }

    pub fn clear(&mut self) {
        self.sentence.clear();
        self.waiting.clear();
        self.pieces = 0;
    }

    fn finish(&mut self, emit: &mut impl FnMut(Language, &str, &str, &str)) -> Option<String> {
        let sentence = std::mem::take(&mut self.sentence);
        let pieces = std::mem::take(&mut self.pieces);
        let example = sentence.trim();
        for (language, word, source) in self.waiting.drain(..) {
            if example.contains(source.as_str()) {
                emit(language, &word, &source, example);
            }
        }
        let hanzi = example.chars().filter(|c| is_han(*c)).count();
        (pieces >= 2 && hanzi >= MIN_SENTENCE_HANZI).then(|| example.to_owned())
    }
}

fn is_sentence_end(c: char) -> bool {
    matches!(c, '。' | '！' | '？' | '!' | '?' | '…' | '\n')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(collector: &mut ExampleCollector, text: &str) -> Vec<(String, String, String)> {
        let mut out = Vec::new();
        let _ = collector.push_text(text, |_, word, source, example| {
            out.push((word.to_owned(), source.to_owned(), example.to_owned()));
        });
        out
    }

    #[test]
    fn hands_over_the_whole_sentence_once_it_ends() {
        let mut collector = ExampleCollector::default();
        assert!(collect(&mut collector, "上一句。我们").is_empty());
        collector.note_word(Language::English, "develop", "开发");
        collector.note_word(Language::English, "exploit", "开发");
        assert!(collect(&mut collector, "开发").is_empty());
        assert!(collect(&mut collector, "了一个").is_empty());
        let done = collect(&mut collector, "输入法。下一句");
        assert_eq!(
            done,
            [
                (
                    "develop".into(),
                    "开发".into(),
                    "我们开发了一个输入法。".into()
                ),
                (
                    "exploit".into(),
                    "开发".into(),
                    "我们开发了一个输入法。".into()
                ),
            ]
        );
        assert!(collect(&mut collector, "。").is_empty());
    }

    #[test]
    fn breaks_off_long_sentences_and_lost_context() {
        let mut collector = ExampleCollector::default();
        collector.note_word(Language::English, "world", "世界");
        let long = format!("世界{}", "很".repeat(MAX_SENTENCE));
        let done = collect(&mut collector, &long);
        assert_eq!(done.len(), 1);
        assert_eq!(done[0].2, long);

        collector.note_word(Language::English, "hello", "你好");
        collect(&mut collector, "你好");
        let mut out = Vec::new();
        let _ = collector
            .break_off(|_, word, _, example| out.push((word.to_owned(), example.to_owned())));
        assert_eq!(out, [("hello".to_owned(), "你好".to_owned())]);
    }

    #[test]
    fn keeps_only_a_bounded_lead_in_while_nothing_is_waiting() {
        let mut collector = ExampleCollector::default();
        collect(&mut collector, &"字".repeat(MAX_SENTENCE * 3));
        assert_eq!(collector.sentence.chars().count(), MAX_SENTENCE);
    }

    #[test]
    fn sentences_typed_in_several_pieces_come_back_for_translation() {
        let mut collector = ExampleCollector::default();
        let none = |_: Language, _: &str, _: &str, _: &str| {};
        assert!(collector.push_text("词组", none).is_empty());
        assert!(collector.push_text("没翻译", none).is_empty());
        assert_eq!(collector.push_text("？", none), ["词组没翻译？"]);
        // 一次就上屏的整句不交回（上屏那边已经翻了）；太短的也不交
        assert!(collector.push_text("我们开发了输入法。", none).is_empty());
        collector.push_text("好", none);
        assert!(collector.push_text("的。", none).is_empty());
        collector.push_text("先", none);
        collector.push_text("开发", none);
        collector.push_text("输入法", none);
        assert_eq!(collector.break_off(none).as_deref(), Some("先开发输入法"));
    }
}
