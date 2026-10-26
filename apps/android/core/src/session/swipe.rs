//! 滑行输入：轨迹交 Core 的解码，解出来的字母像一个个点出来的一样进组句。

use qingjian_core::{SwipeKey, SwipeLexicon, SwipePoint};

use super::Session;
use crate::frame::Frame;

impl Session {
    /// 按英文词解出最可能的字母串；解不出是空串。字母直接上屏（英文键盘不组句、Shift 亮着）时壳用它。
    pub fn decode_swipe(&self, trace: &[SwipePoint], keys: &[SwipeKey]) -> String {
        self.decode_with(trace, keys, self.english_lexicon())
    }

    /// 滑完一笔：解出的字母依次进组句（日语进假名），回之后的一帧；候选、选词与点出来的完全一样。
    /// 中文按拼音的音节串解，英文模式按英文词；日语还没有词可比，按拐点取字母。
    pub fn swipe(&mut self, trace: &[SwipePoint], keys: &[SwipeKey]) -> Frame {
        let letters = if self.kana.is_some() {
            self.decode_with(trace, keys, &SwipeLexicon::default())
        } else if self.engine.english_mode() {
            self.decode_with(trace, keys, self.english_lexicon())
        } else {
            let pinyin = self
                .swipe_pinyin
                .get_or_init(|| SwipeLexicon::from_dictionary(self.engine.dictionary()));
            self.decode_with(trace, keys, pinyin)
        };
        for letter in letters.chars() {
            match &mut self.kana {
                Some(kana) => {
                    kana.push(letter);
                }
                None => {
                    self.engine.push(letter);
                }
            }
        }
        self.refresh(String::new())
    }

    fn english_lexicon(&self) -> &SwipeLexicon {
        self.swipe_english.get_or_init(|| {
            self.engine
                .english_words()
                .map(SwipeLexicon::from_word_list)
                .unwrap_or_default()
        })
    }

    fn decode_with(
        &self,
        trace: &[SwipePoint],
        keys: &[SwipeKey],
        lexicon: &SwipeLexicon,
    ) -> String {
        self.swipe_decoder
            .decode(trace, keys, lexicon)
            .into_iter()
            .next()
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use qingjian_core::dictionary::{Dictionary, WordList};
    use qingjian_core::{Engine, SwipeKey, SwipePoint};

    use super::Session;

    fn session() -> Session {
        let dictionary = Dictionary::parse(
            "你好\tni hao\t5000\n鸟\tniao\t3000\n你\tni\t9000\n好\thao\t8000\n中国\tzhong guo\t6000\n",
        )
        .unwrap();
        let english =
            WordList::parse("the\tthe\t7730\nhello\thello\t4720\nhell\thell\t5100\n").unwrap();
        let engine = Engine::new(dictionary).with_english(english);
        Session::new(engine, PathBuf::new(), PathBuf::new(), PathBuf::new())
    }

    fn keys() -> Vec<SwipeKey> {
        let rows = [("qwertyuiop", 0.0), ("asdfghjkl", 0.5), ("zxcvbnm", 1.5)];
        rows.iter()
            .enumerate()
            .flat_map(|(row, (letters, indent))| {
                letters
                    .chars()
                    .enumerate()
                    .map(move |(column, letter)| SwipeKey {
                        letter,
                        x: (indent + column as f32 + 0.5) * 100.0,
                        y: (row as f32 + 0.5) * 140.0,
                        width: 100.0,
                        height: 140.0,
                    })
            })
            .collect()
    }

    /// 依次经过各字母键中心的折线，只取拐点（解码自己重采样）。
    fn trace(word: &str) -> Vec<SwipePoint> {
        let keys = keys();
        word.chars()
            .map(|letter| {
                let key = keys.iter().find(|key| key.letter == letter).unwrap();
                SwipePoint { x: key.x, y: key.y }
            })
            .collect()
    }

    #[test]
    fn chinese_decodes_pinyin_into_the_composition() {
        let mut session = session();
        let frame = session.swipe(&trace("nihao"), &keys());
        assert_eq!(frame.candidates[0].text, "你好");
    }

    #[test]
    fn english_mode_and_direct_letters_decode_english_words() {
        let mut session = session();
        assert_eq!(session.decode_swipe(&trace("hello"), &keys()), "hello");
        session.set_english_mode(true);
        session.swipe(&trace("the"), &keys());
        assert_eq!(session.engine.composition().text(), "the");
    }
}
